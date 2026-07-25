//! WebGPU client: mounted self view, input session, optional debug tools.

#![cfg(target_arch = "wasm32")]

mod body_hit;
mod client;
#[cfg(feature = "debug-tools")]
mod debug;
mod emote_wheel;
mod fire_fx;
mod hit_marker;
mod input;
#[cfg(feature = "debug-tools")]
mod lineup;
mod mesh;
mod mp;
mod pack;
mod remote_present;
mod renderer;
mod reticle;
mod self_present;
mod ui_overlay;
mod view;

use std::cell::RefCell;
use std::rc::Rc;

use game_sim::SelfState;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[cfg(feature = "debug-tools")]
use client::load::maybe_kick_lineup_load;
use client::load::{maybe_kick_remote_loads, maybe_kick_self_load};
pub(crate) use client::ClientInner;
#[cfg(feature = "debug-tools")]
use debug::{DebugHost, DebugTools};
use input::install_input_handlers;
use renderer::{canvas_buffer_size, Renderer};
use ui_overlay::UiOverlay;
use view::ViewController;

type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

#[wasm_bindgen]
pub struct GameClient {
    inner: Rc<RefCell<ClientInner>>,
}

#[wasm_bindgen]
impl GameClient {
    #[wasm_bindgen(js_name = create)]
    pub async fn create(canvas: HtmlCanvasElement) -> Result<GameClient, JsValue> {
        console_error_panic_hook::set_once();

        let renderer = Renderer::new(canvas.clone()).await?;
        let self_state = SelfState::default_loadout();
        let view = ViewController::new();
        renderer.write_view_proj(view.view_matrix(&self_state));

        #[cfg(feature = "debug-tools")]
        let debug = DebugTools::new();
        let ui = UiOverlay::new(&renderer.device, renderer.config.format);

        let ppp = canvas_buffer_size(&canvas, renderer.max_texture_dim).2;
        let inner = Rc::new(RefCell::new(ClientInner::new(
            renderer,
            canvas,
            ppp,
            self_state,
            view,
            ui,
            #[cfg(feature = "debug-tools")]
            debug,
        )));

        web_sys::console::log_1(
            &"WebGPU initialized; self mount ready (click canvas to capture input)".into(),
        );
        Ok(GameClient { inner })
    }

    #[cfg(feature = "debug-tools")]
    #[wasm_bindgen(js_name = debugHost)]
    pub fn debug_host(&self) -> DebugHost {
        DebugHost::new(self.inner.clone())
    }

    #[wasm_bindgen(js_name = renderFrame)]
    pub fn render_frame(&mut self) -> Result<(), JsValue> {
        self.inner.borrow_mut().render_frame()
    }

    #[wasm_bindgen(js_name = startRenderLoop)]
    pub fn start_render_loop(&self) -> Result<(), JsValue> {
        let canvas = self.inner.borrow().canvas.clone();
        install_input_handlers(self.inner.clone(), &canvas);

        let client = self.inner.clone();
        let frame_cb: FrameCallback = Rc::new(RefCell::new(None));
        let frame_cb_clone = frame_cb.clone();

        *frame_cb.borrow_mut() = Some(Closure::new(move || {
            maybe_kick_self_load(&client);
            maybe_kick_remote_loads(&client);
            #[cfg(feature = "debug-tools")]
            maybe_kick_lineup_load(&client);

            {
                let mut client = client.borrow_mut();
                if let Err(err) = client.render_frame() {
                    web_sys::console::error_1(&err);
                }
            }

            if let Some(window) = web_sys::window() {
                let cb = frame_cb_clone.borrow();
                if let Some(closure) = cb.as_ref() {
                    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                }
            }
        }));

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window available"))?;
        {
            let cb = frame_cb.borrow();
            let closure = cb
                .as_ref()
                .ok_or_else(|| JsValue::from_str("Render loop closure missing"))?;
            window
                .request_animation_frame(closure.as_ref().unchecked_ref())
                .map_err(|e| JsValue::from_str(&format!("Failed to schedule frame: {e:?}")))?;
        }

        std::mem::forget(frame_cb);
        Ok(())
    }
}
