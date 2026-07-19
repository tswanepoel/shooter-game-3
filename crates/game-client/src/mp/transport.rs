//! Browser WebSocket adapter (binary frames only).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{BinaryType, MessageEvent, WebSocket};

/// Default game-server port (Vite serves the page on 3000).
pub const DEFAULT_SERVER_PORT: u16 = 9090;

#[derive(Debug)]
pub enum TransportEvent {
    Open,
    Close,
    Error,
    Binary(Vec<u8>),
}

/// Shared event buffer filled by WS callbacks.
type EventBuf = Rc<RefCell<VecDeque<TransportEvent>>>;

pub struct MpTransport {
    socket: Option<WebSocket>,
    events: EventBuf,
    /// Closures kept alive for the socket lifetime.
    _on_open: Option<Closure<dyn FnMut(JsValue)>>,
    _on_close: Option<Closure<dyn FnMut(JsValue)>>,
    _on_error: Option<Closure<dyn FnMut(JsValue)>>,
    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
}

impl MpTransport {
    pub fn new() -> Self {
        Self {
            socket: None,
            events: Rc::new(RefCell::new(VecDeque::new())),
            _on_open: None,
            _on_close: None,
            _on_error: None,
            _on_message: None,
        }
    }

    pub fn connected(&self) -> bool {
        self.socket
            .as_ref()
            .map(|s| s.ready_state() == WebSocket::OPEN)
            .unwrap_or(false)
    }

    /// `ws://{host}:{port}/` — host defaults to page hostname.
    pub fn connect(&mut self, url: &str) -> Result<(), JsValue> {
        self.close();
        let ws = WebSocket::new(url)?;
        ws.set_binary_type(BinaryType::Arraybuffer);

        let events = Rc::clone(&self.events);
        let on_open = Closure::wrap(Box::new({
            let events = Rc::clone(&events);
            move |_e: JsValue| {
                events.borrow_mut().push_back(TransportEvent::Open);
            }
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        let on_close = Closure::wrap(Box::new({
            let events = Rc::clone(&events);
            move |_e: JsValue| {
                events.borrow_mut().push_back(TransportEvent::Close);
            }
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        let on_error = Closure::wrap(Box::new({
            let events = Rc::clone(&events);
            move |_e: JsValue| {
                events.borrow_mut().push_back(TransportEvent::Error);
            }
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        let on_message = Closure::wrap(Box::new({
            let events = Rc::clone(&events);
            move |e: MessageEvent| {
                if let Ok(buf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let arr = js_sys::Uint8Array::new(&buf);
                    let mut bytes = vec![0u8; arr.length() as usize];
                    arr.copy_to(&mut bytes);
                    events.borrow_mut().push_back(TransportEvent::Binary(bytes));
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        self._on_open = Some(on_open);
        self._on_close = Some(on_close);
        self._on_error = Some(on_error);
        self._on_message = Some(on_message);
        self.socket = Some(ws);
        Ok(())
    }

    pub fn close(&mut self) {
        if let Some(ws) = self.socket.take() {
            let _ = ws.close();
            ws.set_onopen(None);
            ws.set_onclose(None);
            ws.set_onerror(None);
            ws.set_onmessage(None);
        }
        self._on_open = None;
        self._on_close = None;
        self._on_error = None;
        self._on_message = None;
    }

    pub fn send_binary(&self, bytes: &[u8]) -> Result<(), JsValue> {
        let Some(ws) = self.socket.as_ref() else {
            return Err(JsValue::from_str("mp transport: no socket"));
        };
        if ws.ready_state() != WebSocket::OPEN {
            return Err(JsValue::from_str("mp transport: not open"));
        }
        ws.send_with_u8_array(bytes)?;
        Ok(())
    }

    pub fn poll_events(&mut self) -> Vec<TransportEvent> {
        self.events.borrow_mut().drain(..).collect()
    }
}

impl Default for MpTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Build default `ws://hostname:9090/` from the browser location.
pub fn default_ws_url() -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let location = window.location();
    let host = location.hostname()?;
    Ok(format!("ws://{host}:{DEFAULT_SERVER_PORT}/"))
}
