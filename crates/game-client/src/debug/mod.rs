//! In-engine debug subsystem: commands, cvars, console shell, host bridge.
//!
//! Feature-gated (`debug-tools`). Transports (egui, JS) only invoke the registry.

mod host;
mod registry;
mod shell;

pub use host::DebugHost;
pub use registry::DebugRegistry;
pub use shell::{DebugShell, OverlayGpu};

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, KeyboardEvent, MouseEvent, WheelEvent};

use crate::ClientInner;

/// Full debug stack owned by the client when `debug-tools` is enabled.
pub struct DebugTools {
    pub registry: DebugRegistry,
    pub shell: DebugShell,
    pending_events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    toggle_requested: bool,
    screenshot_requested: bool,
}

impl DebugTools {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let mut registry = DebugRegistry::new();
        registry.register_defaults();
        Self {
            registry,
            shell: DebugShell::new(device, surface_format),
            pending_events: Vec::new(),
            modifiers: egui::Modifiers::default(),
            toggle_requested: false,
            screenshot_requested: false,
        }
    }

    pub fn execute(&mut self, line: &str) -> String {
        let out = self.registry.execute(line);
        if out == "__REQUEST_SCREENSHOT__" {
            self.request_screenshot();
            let msg = "screenshot queued".to_string();
            self.shell.push_log(msg.clone());
            return msg;
        }
        if !out.is_empty() {
            self.shell.push_log(out.clone());
        }
        out
    }

    pub fn request_screenshot(&mut self) {
        self.screenshot_requested = true;
    }

    pub fn take_screenshot_request(&mut self) -> bool {
        let v = self.screenshot_requested;
        self.screenshot_requested = false;
        v
    }

    pub fn draw_grid(&self) -> bool {
        self.registry.get_bool("draw.grid").unwrap_or(true)
    }

    pub fn is_open(&self) -> bool {
        self.shell.open
    }

    pub fn request_toggle(&mut self) {
        self.toggle_requested = true;
    }

    fn take_toggle(&mut self) -> bool {
        let t = self.toggle_requested;
        self.toggle_requested = false;
        t
    }

    pub fn push_event(&mut self, event: egui::Event) {
        self.pending_events.push(event);
    }

    pub fn set_modifiers(&mut self, modifiers: egui::Modifiers) {
        self.modifiers = modifiers;
    }

    pub fn modifiers(&self) -> egui::Modifiers {
        self.modifiers
    }

    /// Apply backtick toggle, then build egui raw input for this frame.
    pub fn take_raw_input(&mut self, screen_w: f32, screen_h: f32, time: f64) -> egui::RawInput {
        if self.take_toggle() {
            self.shell.open = !self.shell.open;
            if self.shell.open {
                self.shell.focus_input = true;
            }
        }

        let events = std::mem::take(&mut self.pending_events);
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(screen_w, screen_h),
            )),
            time: Some(time),
            predicted_dt: 1.0 / 60.0,
            modifiers: self.modifiers,
            events,
            focused: true,
            max_texture_side: Some(2048),
            ..Default::default()
        }
    }
}

/// Install window-level input listeners that feed the shared client debug stack.
pub fn install_input_handlers(inner: Rc<RefCell<ClientInner>>, canvas: &HtmlCanvasElement) {
    let window = web_sys::window().expect("window");

    {
        let inner = inner.clone();
        let on_key_down = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            let mut client = inner.borrow_mut();
            let debug = &mut client.debug;

            update_modifiers(debug, &event);

            // ` / Backquote — ignore key-repeat so hold does not flicker open/close.
            if event.code() == "Backquote" || event.key() == "`" {
                event.prevent_default();
                if !event.repeat() {
                    debug.request_toggle();
                }
                return;
            }

            // F9 — screenshot (works with console closed or open).
            if event.code() == "F9" {
                event.prevent_default();
                if !event.repeat() {
                    debug.request_screenshot();
                }
                return;
            }

            if !debug.is_open() {
                return;
            }

            event.prevent_default();
            event.stop_propagation();

            if let Some(key) = map_egui_key(&event.code()) {
                debug.push_event(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: event.repeat(),
                    modifiers: debug.modifiers(),
                });
            }

            let key_str = event.key();
            if key_str.chars().count() == 1 {
                if let Some(ch) = key_str.chars().next() {
                    if !ch.is_control() {
                        debug.push_event(egui::Event::Text(ch.to_string()));
                    }
                }
            }
        });
        window
            .add_event_listener_with_callback("keydown", on_key_down.as_ref().unchecked_ref())
            .expect("keydown listener");
        on_key_down.forget();
    }

    {
        let inner = inner.clone();
        let on_key_up = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            let mut client = inner.borrow_mut();
            let debug = &mut client.debug;
            update_modifiers(debug, &event);
            if !debug.is_open() {
                return;
            }
            if let Some(key) = map_egui_key(&event.code()) {
                debug.push_event(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: debug.modifiers(),
                });
            }
        });
        window
            .add_event_listener_with_callback("keyup", on_key_up.as_ref().unchecked_ref())
            .expect("keyup listener");
        on_key_up.forget();
    }

    {
        let inner = inner.clone();
        let canvas_el = canvas.clone();
        let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
            let mut client = inner.borrow_mut();
            if !client.debug.is_open() {
                return;
            }
            let ppp = client.pixels_per_point();
            let rect = canvas_el.get_bounding_client_rect();
            let pos = egui::pos2(
                (event.client_x() as f64 - rect.left()) as f32 / ppp,
                (event.client_y() as f64 - rect.top()) as f32 / ppp,
            );
            client.debug.push_event(egui::Event::PointerMoved(pos));
        });
        canvas
            .add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())
            .expect("mousemove");
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
            let ppp = client.pixels_per_point();
            let rect = canvas_el.get_bounding_client_rect();
            let pos = egui::pos2(
                (event.client_x() as f64 - rect.left()) as f32 / ppp,
                (event.client_y() as f64 - rect.top()) as f32 / ppp,
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
            .expect("mousedown");
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
            let ppp = client.pixels_per_point();
            let rect = canvas_el.get_bounding_client_rect();
            let pos = egui::pos2(
                (event.client_x() as f64 - rect.left()) as f32 / ppp,
                (event.client_y() as f64 - rect.top()) as f32 / ppp,
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
            .expect("mouseup");
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
            .expect("wheel");
        on_wheel.forget();
    }
}

fn update_modifiers(debug: &mut DebugTools, event: &KeyboardEvent) {
    debug.set_modifiers(egui::Modifiers {
        alt: event.alt_key(),
        ctrl: event.ctrl_key(),
        shift: event.shift_key(),
        mac_cmd: event.meta_key(),
        command: event.ctrl_key() || event.meta_key(),
    });
}

fn map_pointer_button(button: i16) -> egui::PointerButton {
    match button {
        1 => egui::PointerButton::Middle,
        2 => egui::PointerButton::Secondary,
        _ => egui::PointerButton::Primary,
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
