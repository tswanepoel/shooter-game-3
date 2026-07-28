//! Bridge browser pointer/keyboard events into egui (OS mouse when unlocked, soft pointer when locked).

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, KeyboardEvent, MouseEvent};

use crate::ClientInner;

pub(super) fn update_egui_modifiers(client: &mut ClientInner, event: &KeyboardEvent) {
    let mods = egui::Modifiers {
        alt: event.alt_key(),
        ctrl: event.ctrl_key(),
        shift: event.shift_key(),
        mac_cmd: event.meta_key(),
        command: event.ctrl_key() || event.meta_key(),
    };
    client.ui.set_modifiers(mods);
}

pub(super) fn push_egui_key(client: &mut ClientInner, event: &KeyboardEvent, pressed: bool) {
    let modifiers = client.ui.modifiers();
    if let Some(key) = map_egui_key(&event.code()) {
        client.ui.push_event(egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: event.repeat(),
            modifiers,
        });
    }
    if pressed {
        let key_str = event.key();
        if key_str.chars().count() == 1 {
            if let Some(ch) = key_str.chars().next() {
                if !ch.is_control() {
                    client.ui.push_event(egui::Event::Text(ch.to_string()));
                }
            }
        }
    }
}

fn map_egui_key(code: &str) -> Option<egui::Key> {
    use egui::Key;
    Some(match code {
        "Enter" => Key::Enter,
        "Escape" => Key::Escape,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        "ArrowUp" => Key::ArrowUp,
        "ArrowDown" => Key::ArrowDown,
        "Home" => Key::Home,
        "End" => Key::End,
        "Tab" => Key::Tab,
        "Space" => Key::Space,
        _ => return None,
    })
}

fn map_pointer_button(button: i16) -> egui::PointerButton {
    match button {
        1 => egui::PointerButton::Middle,
        2 => egui::PointerButton::Secondary,
        _ => egui::PointerButton::Primary,
    }
}

/// Absolute OS mouse → egui while the session is inactive (pre-lock / after eject).
pub(super) fn egui_os_pointer(client: &ClientInner) -> bool {
    if client.session.is_active() {
        return false;
    }
    #[cfg(feature = "debug-tools")]
    if client.debug.is_open() {
        return true;
    }
    client.ui.wants_ui_input(client.mp.phase())
}

pub(super) fn push_soft_pointer_button(client: &mut ClientInner, pressed: bool, button: i16) {
    let p = client.soft_pointer.pos();
    let pos = egui::pos2(p.x, p.y);
    let modifiers = client.ui.modifiers();
    client.ui.push_event(egui::Event::PointerButton {
        pos,
        button: map_pointer_button(button),
        pressed,
        modifiers,
    });
}

pub(super) fn install_egui_pointer(inner: Rc<RefCell<ClientInner>>, canvas: &HtmlCanvasElement) {
    {
        let inner = inner.clone();
        let canvas_el = canvas.clone();
        let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let mut client = inner.borrow_mut();
            if !egui_os_pointer(&client) {
                return;
            }
            let rect = canvas_el.get_bounding_client_rect();
            let pos = egui::pos2(
                (event.client_x() as f64 - rect.left()) as f32,
                (event.client_y() as f64 - rect.top()) as f32,
            );
            client
                .soft_pointer
                .set_bounds(rect.width() as f32, rect.height() as f32);
            client.soft_pointer.set_pos(pos.x, pos.y);
            client.ui.push_event(egui::Event::PointerMoved(pos));
        });
        canvas
            .add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())
            .expect("egui mousemove");
        on_move.forget();
    }

    {
        let inner = inner.clone();
        let canvas_el = canvas.clone();
        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let mut client = inner.borrow_mut();
            if !egui_os_pointer(&client) {
                return;
            }
            event.prevent_default();
            event.stop_propagation();
            let rect = canvas_el.get_bounding_client_rect();
            let pos = egui::pos2(
                (event.client_x() as f64 - rect.left()) as f32,
                (event.client_y() as f64 - rect.top()) as f32,
            );
            let button = map_pointer_button(event.button());
            let modifiers = client.ui.modifiers();
            client.ui.push_event(egui::Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers,
            });
        });
        canvas
            .add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref())
            .expect("egui mousedown");
        on_down.forget();
    }

    {
        let inner = inner.clone();
        let canvas_el = canvas.clone();
        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let mut client = inner.borrow_mut();
            if !egui_os_pointer(&client) {
                return;
            }
            let rect = canvas_el.get_bounding_client_rect();
            let pos = egui::pos2(
                (event.client_x() as f64 - rect.left()) as f32,
                (event.client_y() as f64 - rect.top()) as f32,
            );
            let button = map_pointer_button(event.button());
            let modifiers = client.ui.modifiers();
            client.ui.push_event(egui::Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers,
            });
        });
        canvas
            .add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())
            .expect("egui mouseup");
        on_up.forget();
    }
}
