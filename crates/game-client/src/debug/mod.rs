//! In-engine debug subsystem: commands, cvars, console shell, host bridge.
//!
//! Feature-gated (`debug-tools`). Input listeners live in `crate::input`.

mod host;
mod registry;
mod shell;

pub use host::DebugHost;
pub use registry::DebugRegistry;
pub use shell::DebugShell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugHostRequest {
    Screenshot,
    MpJoin,
    MpLeave,
    MpStatus,
    Blaster(u8),
}

pub struct DebugTools {
    pub registry: DebugRegistry,
    pub shell: DebugShell,
    toggle_requested: bool,
    screenshot_requested: bool,
    host_requests: Vec<DebugHostRequest>,
}

impl DebugTools {
    pub fn new() -> Self {
        let mut registry = DebugRegistry::new();
        registry.register_defaults();
        Self {
            registry,
            shell: DebugShell::new(),
            toggle_requested: false,
            screenshot_requested: false,
            host_requests: Vec::new(),
        }
    }

    pub fn execute(
        &mut self,
        line: &str,
        mouse_sens: &mut crate::preferences::MouseSensitivity,
    ) -> String {
        if let Some(out) = crate::preferences::execute_mouse_sens_command(line, mouse_sens) {
            if !out.is_empty() {
                self.shell.push_log(out.clone());
            }
            return out;
        }
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
        self.registry.get_bool("draw.grid").unwrap_or(false)
    }

    pub fn draw_lineup(&self) -> bool {
        self.registry.get_bool("draw.lineup").unwrap_or(false)
    }

    pub fn net_hud(&self) -> bool {
        self.registry.get_bool("hud.net").unwrap_or(true)
    }

    pub fn residual_hud(&self) -> bool {
        self.registry.get_bool("hud.residual").unwrap_or(true)
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

    pub fn apply_toggle(&mut self) {
        if self.toggle_requested {
            self.toggle_requested = false;
            self.shell.open = !self.shell.open;
            if self.shell.open {
                self.shell.focus_input = true;
            }
        }
    }
}

impl Default for DebugTools {
    fn default() -> Self {
        Self::new()
    }
}
