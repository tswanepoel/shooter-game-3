//! Single render view: mount (self) or debug flycam.

use game_sim::SelfState;
#[cfg(feature = "debug-tools")]
use game_sim::OCULAR_ELEV_CAP_RAD;
use glam::{Mat4, Vec3};

/// Default flycam move speed (m/s). Client presentation, not world ground truth.
#[cfg(feature = "debug-tools")]
const FLY_SPEED_M_S: f32 = 6.0;
/// Sprint multiplier while Shift is held.
#[cfg(feature = "debug-tools")]
const FLY_SPRINT_MULT: f32 = 3.0;
/// Mouse look sensitivity (radians per pixel). Shared with mounted ocular.
pub const LOOK_SENS_RAD_PER_PX: f32 = 0.00025;
/// Discard only pathological single-frame pointer spikes (px), e.g. pointer-lock
/// re-entry jumps. Must stay well above real flicks: at 60 Hz, 48 px/frame is only
/// ~7° and freezes the camera on any fast turn. True browser glitches are hundreds+.
pub const LOOK_SPIKE_PX: f32 = 1024.0;
/// Flycam pitch matches mounted look (±90°). View matrix stays stable at the poles.
/// Rest-pose eye before the body mesh reports a posed face point (character-a).
/// Matches calibrated mount / `FACE_OFFSET_HEAD_KIT` at rest: (0, 1.52, 0.27) m.
const DEFAULT_MOUNTED_EYE_M: Vec3 = Vec3::new(0.0, 1.52, 0.27);

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
    yaw: f32,
    pitch: f32,
}

#[cfg(feature = "debug-tools")]
impl FlyPose {
    fn from_self(self_state: &SelfState, eye: Vec3) -> Self {
        Self {
            position: eye,
            yaw: self_state.ocular_yaw,
            pitch: self_state.ocular_pitch,
        }
    }

    fn forward(self) -> Vec3 {
        let cp = self.pitch.cos();
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
    /// Mounted eye from posed head (updated each frame).
    mounted_eye: Vec3,
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
            fly: FlyPose::from_self(&SelfState::default_loadout(), DEFAULT_MOUNTED_EYE_M),
            mounted_eye: DEFAULT_MOUNTED_EYE_M,
        }
    }

    pub fn set_mounted_eye(&mut self, eye: Vec3) {
        self.mounted_eye = eye;
    }

    /// World-space eye currently used for the mounted (FP) view.
    pub fn mounted_eye(&self) -> Vec3 {
        self.mounted_eye
    }

    #[cfg(feature = "debug-tools")]
    pub fn is_flycam(&self) -> bool {
        self.mode == ViewMode::Flycam
    }

    /// Unmount at the given eye (must be the current FP camera position).
    #[cfg(feature = "debug-tools")]
    pub fn enter_flycam(&mut self, self_state: &SelfState, eye: Vec3) {
        if self.mode == ViewMode::Mounted {
            self.mounted_eye = eye;
            self.fly = FlyPose::from_self(self_state, eye);
        }
        self.mode = ViewMode::Flycam;
    }

    #[cfg(feature = "debug-tools")]
    pub fn leave_flycam(&mut self, self_state: &SelfState) {
        self.mode = ViewMode::Mounted;
        self.fly = FlyPose::from_self(self_state, self.mounted_eye);
    }

    /// Sync mode from `cam.fly` after the mounted eye for this frame is known.
    /// Returns a status line if mode changed.
    #[cfg(feature = "debug-tools")]
    pub fn sync_fly_intent(
        &mut self,
        want_fly: bool,
        self_state: &SelfState,
        mounted_eye: Vec3,
    ) -> Option<&'static str> {
        match (want_fly, self.is_flycam()) {
            (true, false) => {
                self.enter_flycam(self_state, mounted_eye);
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
            .clamp(-OCULAR_ELEV_CAP_RAD, OCULAR_ELEV_CAP_RAD);

        let forward = self.fly.forward();
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
        let yaw = match self.mode {
            ViewMode::Mounted => self_state.ocular_yaw,
            #[cfg(feature = "debug-tools")]
            ViewMode::Flycam => self.fly.yaw,
        };
        look_to_stable(eye, forward, yaw)
    }

    pub fn eye_and_forward(&self, self_state: &SelfState) -> (Vec3, Vec3) {
        match self.mode {
            ViewMode::Mounted => (self.mounted_eye, self_state.ocular_forward()),
            #[cfg(feature = "debug-tools")]
            ViewMode::Flycam => (self.fly.position, self.fly.forward()),
        }
    }
}

/// RH look matrix that stays stable at ±90° pitch (world-up parallel to forward).
fn look_to_stable(eye: Vec3, forward: Vec3, yaw: f32) -> Mat4 {
    let forward = forward.normalize_or_zero();
    if forward.length_squared() < 1e-12 {
        return Mat4::look_to_rh(eye, Vec3::Z, Vec3::Y);
    }
    let mut right = Vec3::Y.cross(forward);
    if right.length_squared() < 1e-10 {
        // Straight up/down: rebuild right from yaw (facing +Z at yaw 0 → right +X).
        right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    }
    let right = right.normalize_or_zero();
    let up = forward.cross(right).normalize_or_zero();
    let up = if up.length_squared() < 1e-12 {
        Vec3::Y
    } else {
        up
    };
    Mat4::look_to_rh(eye, forward, up)
}

#[cfg(all(test, feature = "debug-tools"))]
mod tests {
    use super::*;

    #[test]
    fn mount_uses_ocular_looking_plus_z() {
        let v = ViewController::new();
        let s = SelfState::default_loadout();
        let (eye, forward) = v.eye_and_forward(&s);
        assert!((eye - v.mounted_eye).length() < 1e-5);
        assert!(forward.dot(Vec3::Z) > 0.99);
        assert!(!v.is_flycam());
    }

    #[test]
    fn enter_leave_flycam_remounts_self() {
        let mut v = ViewController::new();
        let s = SelfState::default_loadout();
        let eye = v.mounted_eye();
        assert_eq!(v.sync_fly_intent(true, &s, eye), Some("flycam on"));
        assert!(v.is_flycam());
        assert_eq!(
            v.sync_fly_intent(false, &s, eye),
            Some("flycam off (remounted)")
        );
        assert!(!v.is_flycam());
        let (_, forward) = v.eye_and_forward(&s);
        assert!(forward.dot(Vec3::Z) > 0.99);
    }

    #[test]
    fn enter_flycam_matches_fp_eye_exactly() {
        let mut v = ViewController::new();
        let mut s = SelfState::default_loadout();
        s.apply_look(1.0 / 60.0, 0.4, -0.2);
        // Simulate posed face eye (ahead of default rest along look).
        let fp_eye = Vec3::new(0.05, 1.44, 0.22);
        v.set_mounted_eye(fp_eye);
        let (before_eye, before_fwd) = v.eye_and_forward(&s);
        assert!((before_eye - fp_eye).length() < 1e-6);

        v.enter_flycam(&s, fp_eye);
        let (after_eye, after_fwd) = v.eye_and_forward(&s);
        assert!(
            (after_eye - before_eye).length() < 1e-6,
            "fly eye jumped by {} m",
            (after_eye - before_eye).length()
        );
        assert!((after_fwd - before_fwd).length() < 1e-5);
    }

    #[test]
    fn flycam_moves_forward() {
        let mut v = ViewController::new();
        let s = SelfState::default_loadout();
        let z0 = v.mounted_eye.z;
        v.enter_flycam(&s, v.mounted_eye());
        let input = FlyInput {
            forward: true,
            ..Default::default()
        };
        v.update_flycam(1.0, &input, glam::Vec2::ZERO);
        let (eye, _) = v.eye_and_forward(&s);
        assert!(eye.z > z0 + 1.0);
    }

    #[test]
    fn flycam_ws_follows_pitch() {
        let mut v = ViewController::new();
        let s = SelfState::default_loadout();
        v.enter_flycam(&s, v.mounted_eye());
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

    #[test]
    fn view_matrix_stable_at_straight_up() {
        let v = ViewController::new();
        let mut s = SelfState::default_loadout();
        s.ocular_pitch = std::f32::consts::FRAC_PI_2;
        s.sync_pose();
        let m = v.view_matrix(&s);
        assert!(m.is_finite());
        let (_, forward) = v.eye_and_forward(&s);
        assert!(forward.dot(Vec3::Y) > 0.99);
    }

    #[test]
    fn flycam_keeps_full_pitch_from_fp() {
        let mut v = ViewController::new();
        let mut s = SelfState::default_loadout();
        s.ocular_pitch = -OCULAR_ELEV_CAP_RAD;
        s.sync_pose();
        let eye = v.mounted_eye();
        v.enter_flycam(&s, eye);
        // Same clamp path as a free-look frame with no mouse delta.
        v.update_flycam(1.0 / 60.0, &FlyInput::default(), glam::Vec2::ZERO);
        let (_, fwd) = v.eye_and_forward(&s);
        assert!(
            fwd.dot(-Vec3::Y) > 0.999,
            "flycam must keep straight-down from FP, fwd={fwd}"
        );
        assert!((v.fly.pitch + OCULAR_ELEV_CAP_RAD).abs() < 1e-5);
    }
}
