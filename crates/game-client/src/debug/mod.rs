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

/// Side effects that need `ClientInner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugHostRequest {
    Screenshot,
    MpJoin,
    MpLeave,
    MpStatus,
    /// Dev equip letter `a`…`r` (038).
    Blaster(u8),
}

/// Full debug stack owned by the client when `debug-tools` is enabled.
pub struct DebugTools {
    pub registry: DebugRegistry,
    pub shell: DebugShell,
    pending_events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    toggle_requested: bool,
    screenshot_requested: bool,
    host_requests: Vec<DebugHostRequest>,
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
            host_requests: Vec::new(),
        }
    }

    pub fn execute(&mut self, line: &str) -> String {
        let out = self.registry.execute(line);
        let msg = match out.as_str() {
            "__REQUEST_SCREENSHOT__" => {
                self.host_requests.push(DebugHostRequest::Screenshot);
                self.request_screenshot();
                "screenshot queued".to_string()
            }
            "__REQUEST_MP_JOIN__" => {
                self.host_requests.push(DebugHostRequest::MpJoin);
                "mp: join requested".to_string()
            }
            "__REQUEST_MP_LEAVE__" => {
                self.host_requests.push(DebugHostRequest::MpLeave);
                "mp: leave requested".to_string()
            }
            "__REQUEST_MP_STATUS__" => {
                self.host_requests.push(DebugHostRequest::MpStatus);
                String::new()
            }
            other if other.starts_with("__REQUEST_BLASTER_") => {
                let letter = other
                    .strip_prefix("__REQUEST_BLASTER_")
                    .and_then(|s| s.chars().next())
                    .map(|c| c as u8)
                    .unwrap_or(0);
                if (b'a'..=b'r').contains(&letter) {
                    self.host_requests.push(DebugHostRequest::Blaster(letter));
                    String::new()
                } else {
                    "usage: blaster <a-r>".into()
                }
            }
            other if !other.is_empty() => other.to_string(),
            _ => String::new(),
        };
        if !msg.is_empty() {
            self.shell.push_log(msg.clone());
        }
        msg
    }

    pub fn take_host_requests(&mut self) -> Vec<DebugHostRequest> {
        std::mem::take(&mut self.host_requests)
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

    pub fn draw_lineup(&self) -> bool {
        self.registry.get_bool("draw.lineup").unwrap_or(false)
    }

    pub fn net_hud(&self) -> bool {
        self.registry.get_bool("hud.net").unwrap_or(true)
    }

    pub fn kick_hud(&self) -> bool {
        self.registry.get_bool("hud.kick").unwrap_or(true)
    }

    pub fn draw_tracers(&self) -> bool {
        self.registry.get_bool("draw.tracers").unwrap_or(false)
    }

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
