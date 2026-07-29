//! DOM listeners for pointer lock, play keys, look, soft pointer, weapon wheel, and fire.

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Function, Object, Promise, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlCanvasElement, KeyboardEvent, MouseEvent, WheelEvent};

use crate::ClientInner;

use super::egui_bridge::{
    egui_os_pointer, install_egui_pointer, push_egui_key, push_soft_pointer_button,
    update_egui_modifiers,
};
use super::move_input::MoveInput;

/// Prefer raw mouse deltas (no OS accel). Fall back if the browser rejects it.
fn request_pointer_lock_raw(canvas: &HtmlCanvasElement) {
    let opts = Object::new();
    let _ = Reflect::set(
        &opts,
        &JsValue::from_str("unadjustedMovement"),
        &JsValue::TRUE,
    );

    let el: &JsValue = canvas.as_ref();
    let Some(func) = Reflect::get(el, &JsValue::from_str("requestPointerLock"))
        .ok()
        .and_then(|v| v.dyn_into::<Function>().ok())
    else {
        canvas.request_pointer_lock();
        return;
    };

    match func.call1(el, opts.as_ref()) {
        Ok(result) => {
            if let Ok(promise) = result.dyn_into::<Promise>() {
                let canvas_fb = canvas.clone();
                let on_err = Closure::once(move |_err: JsValue| {
                    // NotSupportedError or similar — plain lock still works.
                    canvas_fb.request_pointer_lock();
                });
                let _ = promise.catch(&on_err);
                on_err.forget();
            }
        }
        Err(_) => canvas.request_pointer_lock(),
    }
}

/// Enter fullscreen (if needed), then pointer-lock. Locking in the same turn as
/// `requestFullscreen` is often cancelled by the fullscreen transition — especially
/// noticeable with raw/`unadjustedMovement` high-DPI mice — so lock after settle.
fn enter_session_capture(document: &Document, canvas: &HtmlCanvasElement) {
    if document.fullscreen_element().is_some() {
        request_pointer_lock_raw(canvas);
        return;
    }

    let el: &JsValue = canvas.as_ref();
    if let Some(func) = Reflect::get(el, &JsValue::from_str("requestFullscreen"))
        .ok()
        .and_then(|v| v.dyn_into::<Function>().ok())
    {
        if let Ok(result) = func.call0(el) {
            if let Ok(promise) = result.dyn_into::<Promise>() {
                let canvas_ok = canvas.clone();
                let canvas_err = canvas.clone();
                let on_ok = Closure::once(move |_v: JsValue| {
                    request_pointer_lock_raw(&canvas_ok);
                });
                let on_err = Closure::once(move |_err: JsValue| {
                    // Fullscreen denied — still try lock from this gesture chain.
                    request_pointer_lock_raw(&canvas_err);
                });
                let _ = promise.then(&on_ok).catch(&on_err);
                on_ok.forget();
                on_err.forget();
            } else {
                // Invoked without a Promise — wait for the change event.
                lock_after_fullscreen_change(document, canvas);
            }
            return;
        }
    }

    if let Some(func) = Reflect::get(el, &JsValue::from_str("webkitRequestFullscreen"))
        .ok()
        .and_then(|v| v.dyn_into::<Function>().ok())
    {
        lock_after_fullscreen_change(document, canvas);
        let _ = func.call0(el);
        return;
    }

    request_pointer_lock_raw(canvas);
}

fn lock_after_fullscreen_change(document: &Document, canvas: &HtmlCanvasElement) {
    let document_el = document.clone();
    let canvas_el = canvas.clone();
    let handler = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let handler_cb = handler.clone();
    let on_change = Closure::<dyn FnMut()>::new(move || {
        request_pointer_lock_raw(&canvas_el);
        if let Some(cb) = handler_cb.borrow_mut().take() {
            let _ = document_el.remove_event_listener_with_callback(
                "fullscreenchange",
                cb.as_ref().unchecked_ref(),
            );
            let _ = document_el.remove_event_listener_with_callback(
                "webkitfullscreenchange",
                cb.as_ref().unchecked_ref(),
            );
        }
    });
    document
        .add_event_listener_with_callback("fullscreenchange", on_change.as_ref().unchecked_ref())
        .expect("fullscreenchange");
    let _ = document.add_event_listener_with_callback(
        "webkitfullscreenchange",
        on_change.as_ref().unchecked_ref(),
    );
    *handler.borrow_mut() = Some(on_change);
}

pub fn install_input_handlers(inner: Rc<RefCell<ClientInner>>, canvas: &HtmlCanvasElement) {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");

    {
        let canvas_el = canvas.clone();
        let document_el = document.clone();
        let on_click = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            if event.button() != 0 {
                return;
            }
            enter_session_capture(&document_el, &canvas_el);
        });
        canvas
            .add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())
            .expect("click listener");
        on_click.forget();
    }

    // Session LMB fire (038): held while pointer-locked and soft pointer disarmed.
    {
        let inner = inner.clone();
        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            if event.button() != 0 {
                return;
            }
            let mut client = inner.borrow_mut();
            if client.soft_pointer_armed() {
                push_soft_pointer_button(&mut client, true, event.button());
                return;
            }
            if client.session.is_active() {
                client.move_input.set_fire_held(true);
            }
        });
        window
            .add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref())
            .expect("fire mousedown");
        on_down.forget();
    }
    {
        let inner = inner.clone();
        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            if event.button() != 0 {
                return;
            }
            let mut client = inner.borrow_mut();
            if client.soft_pointer_armed() {
                push_soft_pointer_button(&mut client, false, event.button());
            }
            client.move_input.set_fire_held(false);
        });
        window
            .add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())
            .expect("fire mouseup");
        on_up.forget();
    }

    // Non-primary buttons for soft pointer (egui secondary etc.).
    {
        let inner = inner.clone();
        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            if event.button() == 0 {
                return;
            }
            let mut client = inner.borrow_mut();
            if client.soft_pointer_armed() {
                push_soft_pointer_button(&mut client, true, event.button());
            }
        });
        window
            .add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref())
            .expect("soft mousedown other");
        on_down.forget();
    }
    {
        let inner = inner.clone();
        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            if event.button() == 0 {
                return;
            }
            let mut client = inner.borrow_mut();
            if client.soft_pointer_armed() {
                push_soft_pointer_button(&mut client, false, event.button());
            }
        });
        window
            .add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())
            .expect("soft mouseup other");
        on_up.forget();
    }

    {
        let inner = inner.clone();
        let canvas_el = canvas.clone();
        let document_el = document.clone();
        let on_lock_change = Closure::<dyn FnMut()>::new(move || {
            let active = pointer_locked_to(&document_el, &canvas_el);
            let mut client = inner.borrow_mut();
            let was = client.session.is_active();
            client.session.set_active(active);
            if active && !was {
                // Prefer last OS-synced position; if never moved, start mid-view.
                let p = client.soft_pointer.pos();
                if p.x <= 0.0 && p.y <= 0.0 {
                    client.soft_pointer.center();
                }
            }
            if was && !active {
                client.move_input.clear_keys();
                client.move_input.set_fire_held(false);
                client.move_input.set_emote_held(false);
                client.fly_input.clear_keys();
            }
            if active != was {
                web_sys::console::log_1(
                    &if active {
                        "input session active"
                    } else {
                        "input session inactive (click canvas to resume)"
                    }
                    .into(),
                );
            }
        });
        document
            .add_event_listener_with_callback(
                "pointerlockchange",
                on_lock_change.as_ref().unchecked_ref(),
            )
            .expect("pointerlockchange");
        on_lock_change.forget();
    }

    {
        let inner = inner.clone();
        let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let mut client = inner.borrow_mut();
            if !client.session.is_active() {
                return;
            }
            let sens = client.mouse_sens.multiplier();
            let dx = event.movement_x() as f32 * sens;
            let dy = event.movement_y() as f32 * sens;
            if dx == 0.0 && dy == 0.0 {
                return;
            }
            if client.soft_pointer_armed() {
                client.soft_pointer.add_delta(dx, dy);
                let p = client.soft_pointer.pos();
                let pos = egui::pos2(p.x, p.y);
                client.ui.push_event(egui::Event::PointerMoved(pos));
            } else {
                client.session.add_look_px(dx, dy);
            }
        });
        window
            .add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())
            .expect("session mousemove");
        on_move.forget();
    }

    {
        let inner = inner.clone();
        let on_key_down = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            on_session_key_down(&inner, &event);
        });
        window
            .add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref())
            .expect("keydown listener");
        on_key_down.forget();
    }

    {
        let inner = inner.clone();
        let on_key_up = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            on_session_key_up(&inner, &event);
        });
        window
            .add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref())
            .expect("keyup listener");
        on_key_up.forget();
    }

    {
        let inner = inner.clone();
        let on_wheel = Closure::<dyn FnMut(WheelEvent)>::new(move |event: WheelEvent| {
            let mut client = inner.borrow_mut();

            let ui_wheel = client.soft_pointer_armed() || egui_os_pointer(&client);
            if ui_wheel {
                event.prevent_default();
                let delta = match event.delta_mode() {
                    1 => egui::vec2(event.delta_x() as f32, event.delta_y() as f32) * 8.0,
                    2 => egui::vec2(event.delta_x() as f32, event.delta_y() as f32) * 30.0,
                    _ => egui::vec2(event.delta_x() as f32, event.delta_y() as f32),
                };
                let modifiers = client.ui.modifiers();
                client.ui.push_event(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta,
                    modifiers,
                });
                return;
            }

            if !client.session.is_active() {
                return;
            }

            #[cfg(feature = "debug-tools")]
            let fly = client.view.is_flycam()
                || client.debug.flycam_wanted()
                || client.mp.is_spectating();
            #[cfg(not(feature = "debug-tools"))]
            let fly = client.view.is_flycam() || client.mp.is_spectating();
            if fly {
                return;
            }

            event.prevent_default();
            client.move_input.note_weapon_wheel(event.delta_y());
        });
        canvas
            .add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref())
            .expect("weapon wheel");
        on_wheel.forget();
    }

    install_egui_pointer(inner, canvas);
}

fn pointer_locked_to(document: &Document, canvas: &HtmlCanvasElement) -> bool {
    let _ = canvas;
    document.pointer_lock_element().is_some()
}

fn on_session_key_down(inner: &Rc<RefCell<ClientInner>>, event: &KeyboardEvent) {
    let mut client = inner.borrow_mut();

    update_egui_modifiers(&mut client, event);

    #[cfg(feature = "debug-tools")]
    {
        if event.code() == "Backquote" || event.key() == "`" {
            event.prevent_default();
            if !event.repeat() {
                client.debug.request_toggle();
            }
            return;
        }

        if event.code() == "F9" {
            event.prevent_default();
            if !event.repeat() {
                client.debug.request_screenshot();
            }
            return;
        }

        if event.code() == "F8" {
            event.prevent_default();
            if !event.repeat() {
                client.debug.toggle_flycam_wanted();
            }
            return;
        }

        if client.debug.is_open() {
            event.prevent_default();
            event.stop_propagation();
            push_egui_key(&mut client, event, true);
            return;
        }
    }

    if client.ui.wants_ui_input(client.mp.phase()) {
        event.prevent_default();
        push_egui_key(&mut client, event, true);
        return;
    }

    if !client.session.is_active() {
        return;
    }

    let code = event.code();

    #[cfg(feature = "debug-tools")]
    let fly = client.view.is_flycam() || client.debug.flycam_wanted() || client.mp.is_spectating();
    #[cfg(not(feature = "debug-tools"))]
    let fly = client.view.is_flycam() || client.mp.is_spectating();

    if fly {
        if crate::view::FlyInput::is_fly_key(&code) {
            event.prevent_default();
            client.fly_input.set_key(&code, true);
        }
    } else if MoveInput::is_move_key(&code) {
        event.prevent_default();
        client.move_input.set_key(&code, true);
    } else if MoveInput::is_sprint_key(&code) {
        event.prevent_default();
        if !event.repeat() {
            client.move_input.note_sprint_press();
        }
    } else if code == "Space" {
        event.prevent_default();
        if !event.repeat() {
            client.move_input.note_jump_press();
        }
    } else if MoveInput::is_reload_key(&code) {
        event.prevent_default();
        if !event.repeat() {
            client.move_input.note_reload_press();
        }
    } else if MoveInput::is_emote_key(&code) {
        event.prevent_default();
        if !event.repeat() {
            client.move_input.set_emote_held(true);
        }
    }
}

fn on_session_key_up(inner: &Rc<RefCell<ClientInner>>, event: &KeyboardEvent) {
    let mut client = inner.borrow_mut();

    update_egui_modifiers(&mut client, event);

    #[cfg(feature = "debug-tools")]
    if client.debug.is_open() {
        push_egui_key(&mut client, event, false);
        return;
    }

    if client.ui.wants_ui_input(client.mp.phase()) {
        push_egui_key(&mut client, event, false);
        return;
    }

    if !client.session.is_active() {
        return;
    }

    let code = event.code();

    #[cfg(feature = "debug-tools")]
    let fly = client.view.is_flycam() || client.debug.flycam_wanted() || client.mp.is_spectating();
    #[cfg(not(feature = "debug-tools"))]
    let fly = client.view.is_flycam() || client.mp.is_spectating();

    if fly {
        client.fly_input.set_key(&code, false);
    } else if MoveInput::is_move_key(&code) {
        client.move_input.set_key(&code, false);
    } else if MoveInput::is_emote_key(&code) {
        client.move_input.set_emote_held(false);
    }
}
