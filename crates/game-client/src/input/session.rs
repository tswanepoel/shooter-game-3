//! Pointer-lock look session state.

use glam::Vec2;

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
