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
use crate::map_present::{MapGpu, MapPresentState};
use crate::renderer::MSAA_SAMPLE_COUNT;
use crate::self_present::{SelfGpu, SelfPresentState};
use crate::sfx::{Sfx, SfxState};

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
        // Spawn / character / equip may have set Idle to abort this load.
        if !matches!(c.self_present, SelfPresentState::Loading) {
            return;
        }
        match result {
            Ok(gpu) => {
                // Drop if kit no longer matches (stale load vs spawn loadout).
                let still = c.self_state.character == self_state.character
                    && gpu.covers_loadout(c.self_state.primary, c.self_state.secondary);
                if still {
                    web_sys::console::log_1(&"self: body and blaster ready".into());
                    c.self_present = SelfPresentState::Ready(gpu);
                } else {
                    c.self_present = SelfPresentState::Idle;
                }
            }
            Err(err) => {
                let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                web_sys::console::error_1(&JsValue::from_str(&format!("self load failed: {msg}")));
                c.self_present = SelfPresentState::Failed;
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
        let ids: Vec<_> =
            c.mp.remotes()
                .ids()
                .filter(|id| c.mp.peer_living(*id))
                .collect();
        let samples: Vec<_> =
            c.mp.remotes()
                .samples()
                .filter(|(id, _)| c.mp.peer_living(*id))
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

pub(crate) fn maybe_kick_corpse_loads(inner: &Rc<RefCell<ClientInner>>) {
    let loads = {
        let mut c = inner.borrow_mut();
        if !c.mp.in_room() {
            c.corpse_present.clear();
            return;
        }
        let corpses = c.world_loot.corpses.clone();
        c.corpse_present.plan_loads(&corpses)
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

    for (corpse_id, character) in loads {
        let position = {
            let c = inner.borrow();
            c.world_loot
                .corpses
                .get(&corpse_id)
                .map(|x| (x.position, x.facing))
                .unwrap_or((glam::Vec3::ZERO, 0.0))
        };
        let state = crate::corpse_present::corpse_load_state(character, position.0, position.1);
        let inner = inner.clone();
        let device = device.clone();
        let queue = queue.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = SelfGpu::load(&device, &queue, format, MSAA_SAMPLE_COUNT, &state).await;
            let mut c = inner.borrow_mut();
            match &result {
                Ok(_) => {
                    web_sys::console::log_1(
                        &format!("mp: corpse id={corpse_id} present ready").into(),
                    );
                }
                Err(err) => {
                    let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                    web_sys::console::error_1(
                        &format!("mp: corpse id={corpse_id} load failed: {msg}").into(),
                    );
                }
            }
            c.corpse_present.finish_load(corpse_id, character, result);
        });
    }
}

pub(crate) fn maybe_kick_blaster_drop_loads(inner: &Rc<RefCell<ClientInner>>) {
    let loads = {
        let mut c = inner.borrow_mut();
        if !c.mp.in_room() && c.world_loot.blaster_drops.is_empty() {
            c.blaster_drop_present.clear();
            return;
        }
        let drops = c.world_loot.blaster_drops.clone();
        c.blaster_drop_present.plan_loads(&drops)
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

    for (drop_id, letter, position) in loads {
        let inner = inner.clone();
        let device = device.clone();
        let queue = queue.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = crate::blaster_drop_present::load_floor_blaster(
                &device,
                &queue,
                format,
                MSAA_SAMPLE_COUNT,
                letter,
                position,
            )
            .await;
            let mut c = inner.borrow_mut();
            match &result {
                Ok(_) => {
                    web_sys::console::log_1(
                        &format!(
                            "loot: blaster drop id={drop_id} letter={} ready",
                            letter as char
                        )
                        .into(),
                    );
                }
                Err(err) => {
                    let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                    web_sys::console::error_1(
                        &format!("loot: blaster drop id={drop_id} load failed: {msg}").into(),
                    );
                }
            }
            c.blaster_drop_present.finish_load(drop_id, letter, result);
        });
    }
}

pub(crate) fn maybe_kick_map_load(inner: &Rc<RefCell<ClientInner>>) {
    let should_start = {
        let c = inner.borrow();
        c.mp.match_started()
            && c.mp.match_map() == Some(game_net::DEFAULT_MAP)
            && matches!(c.map_present, MapPresentState::Idle)
    };
    if !should_start {
        let mut c = inner.borrow_mut();
        if (!c.mp.in_room() || !c.mp.match_started())
            && !matches!(c.map_present, MapPresentState::Idle)
        {
            c.map_present = MapPresentState::Idle;
            c.map_world = game_sim::MapWorld::empty();
        }
        return;
    }

    {
        let mut c = inner.borrow_mut();
        c.map_present = MapPresentState::Loading;
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
        let result = MapGpu::load_map_a(&device, &queue, format, MSAA_SAMPLE_COUNT).await;
        let mut c = inner.borrow_mut();
        if !c.mp.match_started() {
            c.map_present = MapPresentState::Idle;
            c.map_world = game_sim::MapWorld::empty();
            return;
        }
        match result {
            Ok((gpu, world)) => {
                web_sys::console::log_1(&"map: map-a ready".into());
                c.map_present = MapPresentState::Ready(gpu);
                c.map_world = world;
            }
            Err(err) => {
                let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                web_sys::console::error_1(&format!("map load failed: {msg}").into());
                c.map_present = MapPresentState::Failed;
                c.map_world = game_sim::MapWorld::empty();
            }
        }
    });
}

pub(crate) fn maybe_kick_sfx_load(inner: &Rc<RefCell<ClientInner>>) {
    let should_start = {
        let c = inner.borrow();
        matches!(c.sfx, SfxState::Idle)
    };
    if !should_start {
        return;
    }

    {
        let mut c = inner.borrow_mut();
        c.sfx = SfxState::Loading;
    }

    let inner = inner.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = Sfx::load().await;
        let mut c = inner.borrow_mut();
        if !matches!(c.sfx, SfxState::Loading) {
            return;
        }
        match result {
            Ok(sfx) => {
                web_sys::console::log_1(&"sfx: bang ready".into());
                c.sfx = SfxState::Ready(sfx);
            }
            Err(err) => {
                let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                web_sys::console::error_1(&JsValue::from_str(&format!("sfx load failed: {msg}")));
                c.sfx = SfxState::Failed;
            }
        }
    });
}

#[cfg(feature = "debug-tools")]
pub(crate) fn maybe_kick_lineup_load(inner: &Rc<RefCell<ClientInner>>) {
    {
        let mut c = inner.borrow_mut();
        if !c.debug.draw_lineup() {
            if matches!(c.lineup, LineupState::Failed) {
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
                c.lineup = LineupState::Failed;
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
