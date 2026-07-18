//! Single render view: mount (self) or debug flycam.

use game_sim::SelfState;
use glam::{Mat4, Vec3};

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
    Mounted,
    #[cfg(feature = "debug-tools")]
    Flycam,
}

#[cfg(feature = "debug-tools")]
#[derive(Debug, Clone, Copy)]
struct FlyPose {
    position: Vec3,
    /// Yaw around Y (rad). 0 looks along **+Z**.
    yaw: f32,
    pitch: f32,
}

#[cfg(feature = "debug-tools")]
impl FlyPose {
    fn from_self(self_state: &SelfState) -> Self {
        Self {
            position: self_state.eye_world(),
            yaw: self_state.yaw,
            pitch: 0.0,
        }
    }

    fn forward(self) -> Vec3 {
        let cp = self.pitch.cos();
        // Match SelfState::facing at pitch 0: (sin yaw, 0, cos yaw).
        Vec3::new(self.yaw.sin() * cp, self.pitch.sin(), self.yaw.cos() * cp)
    }
}

/// Held-key input for the flycam controller (look comes from the input session).
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
}

#[cfg(feature = "debug-tools")]
impl FlyInput {
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
            fly: FlyPose::from_self(&SelfState::default_loadout()),
        }
    }

    #[cfg(feature = "debug-tools")]
    pub fn is_flycam(&self) -> bool {
        self.mode == ViewMode::Flycam
    }

    #[cfg(feature = "debug-tools")]
    pub fn enter_flycam(&mut self, self_state: &SelfState) {
        if self.mode == ViewMode::Mounted {
            self.fly = FlyPose::from_self(self_state);
        }
        self.mode = ViewMode::Flycam;
    }

    #[cfg(feature = "debug-tools")]
    pub fn leave_flycam(&mut self, self_state: &SelfState) {
        self.mode = ViewMode::Mounted;
        self.fly = FlyPose::from_self(self_state);
    }

    /// Sync mode from `cam.fly`. Returns a status line if mode changed.
    #[cfg(feature = "debug-tools")]
    pub fn sync_fly_intent(
        &mut self,
        want_fly: bool,
        self_state: &SelfState,
    ) -> Option<&'static str> {
        match (want_fly, self.is_flycam()) {
            (true, false) => {
                self.enter_flycam(self_state);
                Some("flycam on")
            }
            (false, true) => {
                self.leave_flycam(self_state);
                Some("flycam off (remounted)")
            }
            _ => None,
        }
    }

    #[cfg(feature = "debug-tools")]
    pub fn update_flycam(&mut self, dt: f32, input: &FlyInput, look_px: glam::Vec2) {
        if self.mode != ViewMode::Flycam {
            return;
        }

        self.fly.yaw -= look_px.x * LOOK_SENS_RAD_PER_PX;
        self.fly.pitch = (self.fly.pitch - look_px.y * LOOK_SENS_RAD_PER_PX)
            .clamp(-MAX_PITCH_RAD, MAX_PITCH_RAD);

        let forward = self.fly.forward();
        // Strafe stays level; W/S follow the look axis (including pitch) for free-fly inspect.
        let flat_right = Vec3::new(forward.x, 0.0, forward.z)
            .normalize_or_zero()
            .cross(Vec3::Y)
            .normalize_or_zero();

        let mut wish = Vec3::ZERO;
        if input.forward {
            wish += forward;
        }
        if input.back {
            wish -= forward;
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

    pub fn view_matrix(&self, self_state: &SelfState) -> Mat4 {
        let (eye, forward) = self.eye_and_forward(self_state);
        Mat4::look_to_rh(eye, forward, Vec3::Y)
    }

    fn eye_and_forward(&self, self_state: &SelfState) -> (Vec3, Vec3) {
        match self.mode {
            ViewMode::Mounted => (self_state.eye_world(), self_state.facing()),
            #[cfg(feature = "debug-tools")]
            ViewMode::Flycam => (self.fly.position, self.fly.forward()),
        }
    }
}

#[cfg(all(test, feature = "debug-tools"))]
mod tests {
    use super::*;

    #[test]
    fn mount_uses_self_eye_looking_plus_z() {
        let v = ViewController::new();
        let s = SelfState::default_loadout();
        let (eye, forward) = v.eye_and_forward(&s);
        assert!((eye - s.eye_world()).length() < 1e-5);
        assert!(forward.dot(Vec3::Z) > 0.99);
        assert!(!v.is_flycam());
    }

    #[test]
    fn enter_leave_flycam_remounts_self() {
        let mut v = ViewController::new();
        let s = SelfState::default_loadout();
        assert_eq!(v.sync_fly_intent(true, &s), Some("flycam on"));
        assert!(v.is_flycam());
        assert_eq!(v.sync_fly_intent(false, &s), Some("flycam off (remounted)"));
        assert!(!v.is_flycam());
        let (eye, forward) = v.eye_and_forward(&s);
        assert!((eye - s.eye_world()).length() < 1e-5);
        assert!(forward.dot(Vec3::Z) > 0.99);
    }

    #[test]
    fn flycam_moves_forward() {
        let mut v = ViewController::new();
        let s = SelfState::default_loadout();
        v.enter_flycam(&s);
        let input = FlyInput {
            forward: true,
            ..Default::default()
        };
        v.update_flycam(1.0, &input, glam::Vec2::ZERO);
        let (eye, _) = v.eye_and_forward(&s);
        // Looking +Z: forward move increases z.
        assert!(eye.z > s.eye_world().z + 1.0);
    }

    #[test]
    fn flycam_ws_follows_pitch() {
        let mut v = ViewController::new();
        let s = SelfState::default_loadout();
        v.enter_flycam(&s);
        // Pitch up ~45°, then hold W for 1s — should gain altitude.
        v.update_flycam(0.0, &FlyInput::default(), glam::Vec2::new(0.0, -400.0));
        let y0 = v.eye_and_forward(&s).0.y;
        v.update_flycam(
            1.0,
            &FlyInput {
                forward: true,
                ..Default::default()
            },
            glam::Vec2::ZERO,
        );
        let y1 = v.eye_and_forward(&s).0.y;
        assert!(
            y1 > y0 + 1.0,
            "expected look-relative climb, y0={y0} y1={y1}"
        );
    }
}
