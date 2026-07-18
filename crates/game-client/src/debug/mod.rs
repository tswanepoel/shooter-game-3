//! In-engine debug subsystem: commands, cvars, console shell, host bridge.
//!
//! Feature-gated (`debug-tools`). Transports (egui, JS) only invoke the registry.
//! Input listeners live in `crate::input` (shared session; 007).

mod host;
mod registry;
mod shell;

pub use host::DebugHost;
pub use registry::DebugRegistry;
pub use shell::{DebugShell, OverlayGpu};

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

    /// Whether the registry wants flycam (view syncs this each frame).
    pub fn flycam_wanted(&self) -> bool {
        self.registry.get_bool("cam.fly").unwrap_or(false)
    }

    pub fn toggle_flycam_wanted(&mut self) {
        let _ = self.registry.execute("flycam toggle");
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
