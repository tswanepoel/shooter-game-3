//! In-game input session: pointer lock on canvas click; browser eject ends it.

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec2;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
#[cfg(feature = "debug-tools")]
use web_sys::WheelEvent;
use web_sys::{Document, HtmlCanvasElement, KeyboardEvent, MouseEvent};

use crate::ClientInner;

/// Held WASD for mounted walk (016). Look-relative axes; no arrows.
#[derive(Debug, Default, Clone)]
pub struct MoveInput {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
}

impl MoveInput {
    pub fn set_key(&mut self, code: &str, pressed: bool) {
        match code {
            "KeyW" => self.forward = pressed,
            "KeyS" => self.back = pressed,
            "KeyA" => self.left = pressed,
            "KeyD" => self.right = pressed,
            _ => {}
        }
    }

    pub fn is_move_key(code: &str) -> bool {
        matches!(code, "KeyW" | "KeyA" | "KeyS" | "KeyD")
    }

    pub fn clear_keys(&mut self) {
        *self = Self::default();
    }

    /// Digital forward / strafe in −1…1 (W/S and A/D).
    pub fn axes(&self) -> (f32, f32) {
        let forward = (self.forward as i8 - self.back as i8) as f32;
        let strafe = (self.right as i8 - self.left as i8) as f32;
        (forward, strafe)
    }
}

#[derive(Debug, Default)]
pub struct InputSession {
    active: bool,
    look_px: Vec2,
}

impl InputSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        if self.active && !active {
            self.look_px = Vec2::ZERO;
        }
        self.active = active;
    }

    pub fn add_look_px(&mut self, dx: f32, dy: f32) {
        if self.active {
            self.look_px.x += dx;
            self.look_px.y += dy;
        }
    }

    pub fn take_look_px(&mut self) -> Vec2 {
        let d = self.look_px;
        self.look_px = Vec2::ZERO;
        d
    }
}

pub fn install_input_handlers(inner: Rc<RefCell<ClientInner>>, canvas: &HtmlCanvasElement) {
    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");

    {
        let canvas_el = canvas.clone();
        let on_click = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            if event.button() != 0 {
                return;
            }
            canvas_el.request_pointer_lock();
        });
        canvas
            .add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())
            .expect("click listener");
        on_click.forget();
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
            if was && !active {
                client.move_input.clear_keys();
                #[cfg(feature = "debug-tools")]
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
            let dx = event.movement_x() as f32;
            let dy = event.movement_y() as f32;
            if dx != 0.0 || dy != 0.0 {
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

    #[cfg(feature = "debug-tools")]
    {
        install_debug_shell_pointer(inner, canvas);
    }
}

fn pointer_locked_to(document: &Document, canvas: &HtmlCanvasElement) -> bool {
    let _ = canvas;
    document.pointer_lock_element().is_some()
}

fn on_session_key_down(inner: &Rc<RefCell<ClientInner>>, event: &KeyboardEvent) {
    let mut client = inner.borrow_mut();

    #[cfg(feature = "debug-tools")]
    {
        update_debug_modifiers(&mut client, event);

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
            let modifiers = client.debug.modifiers();
            if let Some(key) = map_egui_key(&event.code()) {
                client.debug.push_event(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: event.repeat(),
                    modifiers,
                });
            }
            let key_str = event.key();
            if key_str.chars().count() == 1 {
                if let Some(ch) = key_str.chars().next() {
                    if !ch.is_control() {
                        client.debug.push_event(egui::Event::Text(ch.to_string()));
                    }
                }
            }
            return;
        }
    }

    if !client.session.is_active() {
        return;
    }

    let code = event.code();

    #[cfg(feature = "debug-tools")]
    let fly = client.view.is_flycam() || client.debug.flycam_wanted();
    #[cfg(not(feature = "debug-tools"))]
    let fly = false;

    if fly {
        #[cfg(feature = "debug-tools")]
        if crate::view::FlyInput::is_fly_key(&code) {
            event.prevent_default();
            client.fly_input.set_key(&code, true);
        }
    } else if MoveInput::is_move_key(&code) {
        event.prevent_default();
        client.move_input.set_key(&code, true);
    }
}

fn on_session_key_up(inner: &Rc<RefCell<ClientInner>>, event: &KeyboardEvent) {
    let mut client = inner.borrow_mut();

    #[cfg(feature = "debug-tools")]
    {
        update_debug_modifiers(&mut client, event);

        if client.debug.is_open() {
            if let Some(key) = map_egui_key(&event.code()) {
                let modifiers = client.debug.modifiers();
                client.debug.push_event(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers,
                });
            }
            return;
        }
    }

    if !client.session.is_active() {
        return;
    }

    let code = event.code();

    #[cfg(feature = "debug-tools")]
    let fly = client.view.is_flycam() || client.debug.flycam_wanted();
    #[cfg(not(feature = "debug-tools"))]
    let fly = false;

    if fly {
        #[cfg(feature = "debug-tools")]
        client.fly_input.set_key(&code, false);
    } else if MoveInput::is_move_key(&code) {
        client.move_input.set_key(&code, false);
    }
}

#[cfg(feature = "debug-tools")]
fn update_debug_modifiers(client: &mut ClientInner, event: &KeyboardEvent) {
    client.debug.set_modifiers(egui::Modifiers {
        alt: event.alt_key(),
        ctrl: event.ctrl_key(),
        shift: event.shift_key(),
        mac_cmd: event.meta_key(),
        command: event.ctrl_key() || event.meta_key(),
    });
}

#[cfg(feature = "debug-tools")]
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

#[cfg(feature = "debug-tools")]
fn map_pointer_button(button: i16) -> egui::PointerButton {
    match button {
        1 => egui::PointerButton::Middle,
        2 => egui::PointerButton::Secondary,
        _ => egui::PointerButton::Primary,
    }
}

#[cfg(feature = "debug-tools")]
fn install_debug_shell_pointer(inner: Rc<RefCell<ClientInner>>, canvas: &HtmlCanvasElement) {
    {
        let inner = inner.clone();
        let canvas_el = canvas.clone();
        let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let mut client = inner.borrow_mut();
            if !client.debug.is_open() {
                return;
            }
            // CSS px = egui points.
            let rect = canvas_el.get_bounding_client_rect();
            let pos = egui::pos2(
                (event.client_x() as f64 - rect.left()) as f32,
                (event.client_y() as f64 - rect.top()) as f32,
            );
            client.debug.push_event(egui::Event::PointerMoved(pos));
        });
        canvas
            .add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())
            .expect("debug mousemove");
        on_move.forget();
    }

    {
        let inner = inner.clone();
        let canvas_el = canvas.clone();
        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let mut client = inner.borrow_mut();
            if !client.debug.is_open() {
                return;
            }
            event.prevent_default();
            let rect = canvas_el.get_bounding_client_rect();
            let pos = egui::pos2(
                (event.client_x() as f64 - rect.left()) as f32,
                (event.client_y() as f64 - rect.top()) as f32,
            );
            let button = map_pointer_button(event.button());
            let modifiers = client.debug.modifiers();
            client.debug.push_event(egui::Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers,
            });
        });
        canvas
            .add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref())
            .expect("debug mousedown");
        on_down.forget();
    }

    {
        let inner = inner.clone();
        let canvas_el = canvas.clone();
        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let mut client = inner.borrow_mut();
            if !client.debug.is_open() {
                return;
            }
            let rect = canvas_el.get_bounding_client_rect();
            let pos = egui::pos2(
                (event.client_x() as f64 - rect.left()) as f32,
                (event.client_y() as f64 - rect.top()) as f32,
            );
            let button = map_pointer_button(event.button());
            let modifiers = client.debug.modifiers();
            client.debug.push_event(egui::Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers,
            });
        });
        canvas
            .add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())
            .expect("debug mouseup");
        on_up.forget();
    }

    {
        let inner = inner.clone();
        let on_wheel = Closure::<dyn FnMut(WheelEvent)>::new(move |event: WheelEvent| {
            let mut client = inner.borrow_mut();
            if !client.debug.is_open() {
                return;
            }
            event.prevent_default();
            let delta = match event.delta_mode() {
                1 => egui::vec2(event.delta_x() as f32, event.delta_y() as f32) * 8.0,
                2 => egui::vec2(event.delta_x() as f32, event.delta_y() as f32) * 30.0,
                _ => egui::vec2(event.delta_x() as f32, event.delta_y() as f32),
            };
            let modifiers = client.debug.modifiers();
            client.debug.push_event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta,
                modifiers,
            });
        });
        canvas
            .add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref())
            .expect("debug wheel");
        on_wheel.forget();
    }
}
