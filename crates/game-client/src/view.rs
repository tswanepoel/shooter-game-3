//! Single render view: mount (self) or debug flycam controller.
//!
//! Pose source only — not a second camera system. Mount uses shared eye-height
//! ground truth from `game-sim`. Flycam is view-only free inspection.

use game_sim::STANDING_EYE_HEIGHT_M;
use glam::{Mat4, Vec3};

#[cfg(feature = "debug-tools")]
use glam::Vec2;

/// Default flycam move speed (m/s). Client presentation, not world ground truth.
#[cfg(feature = "debug-tools")]
const FLY_SPEED_M_S: f32 = 6.0;
/// Sprint multiplier while Shift is held.
#[cfg(feature = "debug-tools")]
const FLY_SPRINT_MULT: f32 = 3.0;
/// Mouse look sensitivity (radians per pixel).
#[cfg(feature = "debug-tools")]
const LOOK_SENS_RAD_PER_PX: f32 = 0.0025;
/// Pitch clamp so the view never flips.
#[cfg(feature = "debug-tools")]
const MAX_PITCH_RAD: f32 = std::f32::consts::FRAC_PI_2 - 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    /// Attached to self (stub eye pose until a real body exists).
    Mounted,
    /// Debug free-fly; does not move a sim body.
    #[cfg(feature = "debug-tools")]
    Flycam,
}

#[cfg(feature = "debug-tools")]
#[derive(Debug, Clone, Copy)]
struct FlyPose {
    position: Vec3,
    /// Yaw around Y (rad). 0 looks along −Z (same as the stub mount).
    yaw: f32,
    /// Pitch (rad). Positive looks up.
    pitch: f32,
}

#[cfg(feature = "debug-tools")]
impl FlyPose {
    fn from_mount() -> Self {
        Self {
            position: stub_mount_eye(),
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    fn forward(self) -> Vec3 {
        let cp = self.pitch.cos();
        Vec3::new(-self.yaw.sin() * cp, self.pitch.sin(), -self.yaw.cos() * cp)
    }
}

/// Held-key and look-delta input for the flycam controller.
#[cfg(feature = "debug-tools")]
#[derive(Debug, Default, Clone)]
pub struct FlyInput {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
    /// Accumulated mouse delta (pixels) since last update; cleared after apply.
    look_px: Vec2,
}

#[cfg(feature = "debug-tools")]
impl FlyInput {
    pub fn add_look_px(&mut self, dx: f32, dy: f32) {
        self.look_px.x += dx;
        self.look_px.y += dy;
    }

    pub fn set_key(&mut self, code: &str, pressed: bool) {
        match code {
            "KeyW" | "ArrowUp" => self.forward = pressed,
            "KeyS" | "ArrowDown" => self.back = pressed,
            "KeyA" | "ArrowLeft" => self.left = pressed,
            "KeyD" | "ArrowRight" => self.right = pressed,
            "KeyE" | "Space" => self.up = pressed,
            "KeyQ" | "ControlLeft" | "ControlRight" => self.down = pressed,
            "ShiftLeft" | "ShiftRight" => self.sprint = pressed,
            _ => {}
        }
    }

    pub fn is_fly_key(code: &str) -> bool {
        matches!(
            code,
            "KeyW"
                | "KeyA"
                | "KeyS"
                | "KeyD"
                | "KeyQ"
                | "KeyE"
                | "Space"
                | "ControlLeft"
                | "ControlRight"
                | "ShiftLeft"
                | "ShiftRight"
                | "ArrowUp"
                | "ArrowDown"
                | "ArrowLeft"
                | "ArrowRight"
        )
    }

    fn take_look_px(&mut self) -> Vec2 {
        let d = self.look_px;
        self.look_px = Vec2::ZERO;
        d
    }

    /// Clear movement keys (e.g. when leaving flycam or opening the console).
    pub fn clear_keys(&mut self) {
        *self = Self::default();
    }
}

/// One view pose source for the renderer.
#[derive(Debug)]
pub struct ViewController {
    mode: ViewMode,
    #[cfg(feature = "debug-tools")]
    fly: FlyPose,
}

impl Default for ViewController {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewController {
    pub fn new() -> Self {
        Self {
            mode: ViewMode::Mounted,
            #[cfg(feature = "debug-tools")]
            fly: FlyPose::from_mount(),
        }
    }

    #[cfg(feature = "debug-tools")]
    pub fn is_flycam(&self) -> bool {
        self.mode == ViewMode::Flycam
    }

    /// Enter debug flycam, seeding from the stub mount pose.
    #[cfg(feature = "debug-tools")]
    pub fn enter_flycam(&mut self) {
        if self.mode == ViewMode::Mounted {
            self.fly = FlyPose::from_mount();
        }
        self.mode = ViewMode::Flycam;
    }

    /// Leave flycam and remount the self (stub) vantage.
    #[cfg(feature = "debug-tools")]
    pub fn leave_flycam(&mut self) {
        self.mode = ViewMode::Mounted;
        self.fly = FlyPose::from_mount();
    }

    /// Sync mode from the debug `cam.fly` intent. Returns a short status if mode changed.
    #[cfg(feature = "debug-tools")]
    pub fn sync_fly_intent(&mut self, want_fly: bool) -> Option<&'static str> {
        match (want_fly, self.is_flycam()) {
            (true, false) => {
                self.enter_flycam();
                Some("flycam on")
            }
            (false, true) => {
                self.leave_flycam();
                Some("flycam off (remounted)")
            }
            _ => None,
        }
    }

    #[cfg(feature = "debug-tools")]
    pub fn update_flycam(&mut self, dt: f32, input: &mut FlyInput) {
        if self.mode != ViewMode::Flycam {
            return;
        }

        let look = input.take_look_px();
        self.fly.yaw -= look.x * LOOK_SENS_RAD_PER_PX;
        self.fly.pitch =
            (self.fly.pitch - look.y * LOOK_SENS_RAD_PER_PX).clamp(-MAX_PITCH_RAD, MAX_PITCH_RAD);

        let forward = self.fly.forward();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        // Horizontal forward for WASD (stay level when looking up/down).
        let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let flat_right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        let mut wish = Vec3::ZERO;
        if input.forward {
            wish += flat_forward;
        }
        if input.back {
            wish -= flat_forward;
        }
        if input.right {
            wish += flat_right;
        }
        if input.left {
            wish -= flat_right;
        }
        if input.up {
            wish += Vec3::Y;
        }
        if input.down {
            wish -= Vec3::Y;
        }

        if wish.length_squared() > 0.0 {
            let speed = if input.sprint {
                FLY_SPEED_M_S * FLY_SPRINT_MULT
            } else {
                FLY_SPEED_M_S
            };
            self.fly.position += wish.normalize() * speed * dt;
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        let (eye, forward) = self.eye_and_forward();
        Mat4::look_to_rh(eye, forward, Vec3::Y)
    }

    fn eye_and_forward(&self) -> (Vec3, Vec3) {
        match self.mode {
            ViewMode::Mounted => (stub_mount_eye(), stub_mount_forward()),
            #[cfg(feature = "debug-tools")]
            ViewMode::Flycam => (self.fly.position, self.fly.forward()),
        }
    }
}

fn stub_mount_eye() -> Vec3 {
    Vec3::new(0.0, STANDING_EYE_HEIGHT_M, 0.0)
}

fn stub_mount_forward() -> Vec3 {
    Vec3::NEG_Z
}

#[cfg(all(test, feature = "debug-tools"))]
mod tests {
    use super::*;

    #[test]
    fn mount_is_eye_height_looking_neg_z() {
        let v = ViewController::new();
        let (eye, forward) = v.eye_and_forward();
        assert!((eye.y - STANDING_EYE_HEIGHT_M).abs() < 1e-5);
        assert!(forward.dot(Vec3::NEG_Z) > 0.99);
        assert!(!v.is_flycam());
    }

    #[test]
    fn enter_leave_flycam() {
        let mut v = ViewController::new();
        assert_eq!(v.sync_fly_intent(true), Some("flycam on"));
        assert!(v.is_flycam());
        assert_eq!(v.sync_fly_intent(false), Some("flycam off (remounted)"));
        assert!(!v.is_flycam());
        let (eye, _) = v.eye_and_forward();
        assert!((eye.y - STANDING_EYE_HEIGHT_M).abs() < 1e-5);
    }

    #[test]
    fn flycam_moves_forward() {
        let mut v = ViewController::new();
        v.enter_flycam();
        let mut input = FlyInput {
            forward: true,
            ..Default::default()
        };
        v.update_flycam(1.0, &mut input);
        let (eye, _) = v.eye_and_forward();
        // Looking −Z: forward move decreases z.
        assert!(eye.z < -1.0);
    }
}
