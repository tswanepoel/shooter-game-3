//! Async kit load kickers (self, remotes, optional lineup).

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
#[cfg(feature = "debug-tools")]
use wasm_bindgen::JsCast;
#[cfg(feature = "debug-tools")]
use web_sys::HtmlCanvasElement;

#[cfg(feature = "debug-tools")]
use crate::lineup::{LineupGpu, LineupState};
use crate::renderer::MSAA_SAMPLE_COUNT;
use crate::self_present::{SelfGpu, SelfPresentState};

use super::ClientInner;

pub(crate) fn maybe_kick_self_load(inner: &Rc<RefCell<ClientInner>>) {
    let should_start = {
        let c = inner.borrow();
        matches!(c.self_present, SelfPresentState::Idle)
    };
    if !should_start {
        return;
    }

    {
        let mut c = inner.borrow_mut();
        c.self_present = SelfPresentState::Loading;
    }

    let (device, queue, format, self_state) = {
        let c = inner.borrow();
        (
            c.renderer.device.clone(),
            c.renderer.queue.clone(),
            c.renderer.config.format,
            c.self_state.clone(),
        )
    };

    let inner = inner.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = SelfGpu::load(&device, &queue, format, MSAA_SAMPLE_COUNT, &self_state).await;
        let mut c = inner.borrow_mut();
        match result {
            Ok(gpu) => {
                web_sys::console::log_1(&"self: body and blaster ready".into());
                c.self_present = SelfPresentState::Ready(gpu);
            }
            Err(err) => {
                let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                web_sys::console::error_1(&JsValue::from_str(&format!("self load failed: {msg}")));
                c.self_present = SelfPresentState::Failed(msg);
            }
        }
    });
}

pub(crate) fn maybe_kick_remote_loads(inner: &Rc<RefCell<ClientInner>>) {
    let loads = {
        let mut c = inner.borrow_mut();
        if !c.mp.in_room() {
            if c.mp.remotes().count() == 0 {
                c.remote_present.clear();
            }
            return;
        }
        let ids: Vec<_> = c.mp.remotes().ids().collect();
        let samples: Vec<_> =
            c.mp.remotes()
                .samples()
                .map(|(id, s)| (id, s.drive.clone()))
                .collect();
        c.remote_present.plan_loads_from(&ids, &samples)
    };
    if loads.is_empty() {
        return;
    }

    let (device, queue, format) = {
        let c = inner.borrow();
        (
            c.renderer.device.clone(),
            c.renderer.queue.clone(),
            c.renderer.config.format,
        )
    };

    for (id, state, kit) in loads {
        let inner = inner.clone();
        let device = device.clone();
        let queue = queue.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = SelfGpu::load(&device, &queue, format, MSAA_SAMPLE_COUNT, &state).await;
            let mut c = inner.borrow_mut();
            match &result {
                Ok(_) => {
                    web_sys::console::log_1(&format!("mp: remote id={id} present ready").into());
                }
                Err(err) => {
                    let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                    web_sys::console::error_1(
                        &format!("mp: remote id={id} load failed: {msg}").into(),
                    );
                }
            }
            c.remote_present.finish_load(id, kit, result);
        });
    }
}

#[cfg(feature = "debug-tools")]
pub(crate) fn maybe_kick_lineup_load(inner: &Rc<RefCell<ClientInner>>) {
    {
        let mut c = inner.borrow_mut();
        if !c.debug.draw_lineup() {
            if matches!(c.lineup, LineupState::Failed(_)) {
                c.lineup = LineupState::Idle;
            }
            return;
        }
    }

    let should_start = {
        let c = inner.borrow();
        matches!(c.lineup, LineupState::Idle)
    };
    if !should_start {
        return;
    }

    {
        let mut c = inner.borrow_mut();
        c.lineup = LineupState::Loading;
        c.debug.shell.push_log("lineup: loading blaster lineupâ€¦");
    }

    let (device, queue, format) = {
        let c = inner.borrow();
        (
            c.renderer.device.clone(),
            c.renderer.queue.clone(),
            c.renderer.config.format,
        )
    };

    let inner = inner.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = LineupGpu::load(&device, &queue, format, MSAA_SAMPLE_COUNT).await;
        let mut c = inner.borrow_mut();
        match result {
            Ok(gpu) => {
                c.debug.shell.push_log("lineup: ready");
                c.lineup = LineupState::Ready(gpu);
            }
            Err(err) => {
                let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "lineup load failed: {msg}"
                )));
                c.debug.shell.push_log(format!("lineup failed: {msg}"));
                c.lineup = LineupState::Failed(msg);
            }
        }
    });
}

#[cfg(feature = "debug-tools")]
pub(crate) fn capture_canvas_png(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let data_url = canvas
        .to_data_url_with_type("image/png")
        .map_err(|e| JsValue::from_str(&format!("canvas toDataURL failed: {e:?}")))?;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let handler = js_sys::Reflect::get(&window, &JsValue::from_str("__debugSaveShot"))?;
    if handler.is_undefined() || handler.is_null() {
        return Err(JsValue::from_str(
            "window.__debugSaveShot missing (host sink not installed)",
        ));
    }
    let func = handler
        .dyn_into::<js_sys::Function>()
        .map_err(|_| JsValue::from_str("window.__debugSaveShot is not a function"))?;
    let _ = func.call1(&JsValue::NULL, &JsValue::from_str(&data_url))?;
    Ok(())
}
