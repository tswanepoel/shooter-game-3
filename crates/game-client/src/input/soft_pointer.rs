//! Soft pointer: canvas-local absolute cursor under the input session (061).

use glam::Vec2;

#[derive(Debug, Clone)]
pub struct SoftPointer {
    pos: Vec2,
    bounds: Vec2,
}

impl SoftPointer {
    pub fn new() -> Self {
        Self {
            pos: Vec2::ZERO,
            bounds: Vec2::new(1.0, 1.0),
        }
    }

    /// CSS-pixel size of the game view (egui screen space).
    pub fn set_bounds(&mut self, width: f32, height: f32) {
        self.bounds = Vec2::new(width.max(1.0), height.max(1.0));
        self.clamp();
    }

    pub fn center(&mut self) {
        self.pos = self.bounds * 0.5;
    }

    pub fn set_pos(&mut self, x: f32, y: f32) {
        self.pos.x = x;
        self.pos.y = y;
        self.clamp();
    }

    pub fn add_delta(&mut self, dx: f32, dy: f32) {
        self.pos.x += dx;
        self.pos.y += dy;
        self.clamp();
    }

    pub fn pos(&self) -> Vec2 {
        self.pos
    }

    fn clamp(&mut self) {
        self.pos.x = self.pos.x.clamp(0.0, self.bounds.x);
        self.pos.y = self.pos.y.clamp(0.0, self.bounds.y);
    }
}

impl Default for SoftPointer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_clamps_to_bounds() {
        let mut p = SoftPointer::new();
        p.set_bounds(100.0, 50.0);
        p.center();
        assert!((p.pos().x - 50.0).abs() < 1e-5);
        assert!((p.pos().y - 25.0).abs() < 1e-5);
        p.add_delta(1000.0, -1000.0);
        assert!((p.pos().x - 100.0).abs() < 1e-5);
        assert!((p.pos().y - 0.0).abs() < 1e-5);
    }
}
