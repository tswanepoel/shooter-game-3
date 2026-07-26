//! Single render view: mount look (posed head eye socket) or flycam (not view).

use game_sim::{SelfState, LOOK_ELEV_CAP_RAD};
use glam::{Mat4, Vec3};

/// Default flycam move speed (m/s). Client presentation, not world ground truth.
const FLY_SPEED_M_S: f32 = 6.0;
/// Sprint multiplier while Shift is held.
const FLY_SPRINT_MULT: f32 = 3.0;
/// Mouse look sensitivity (radians per pixel). Shared with mounted drive.
pub const LOOK_SENS_RAD_PER_PX: f32 = 0.00015;
/// Flycam pitch matches look elev cap (±90°). View matrix stays stable at the poles.
/// Bootstrap only until present samples look from the posed head.
const DEFAULT_MOUNTED_EYE_M: Vec3 = Vec3::new(0.0, 1.52, 0.27);
const DEFAULT_MOUNTED_FORWARD: Vec3 = Vec3::Z;
/// Spectate entry eye (matches overview loft roughly).
const SPECTATE_START_EYE: Vec3 = Vec3::new(0.0, 8.0, 12.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Mounted,
    Flycam,
}

#[derive(Debug, Clone, Copy)]
struct FlyPose {
    position: Vec3,
    yaw: f32,
    pitch: f32,
}

impl FlyPose {
    fn from_self(self_state: &SelfState, eye: Vec3) -> Self {
        Self {
            position: eye,
            yaw: self_state.look_yaw(),
            pitch: self_state.look_pitch(),
        }
    }

    fn overview_start() -> Self {
        // Look toward origin from loft.
        let eye = SPECTATE_START_EYE;
        let to = -eye;
        let yaw = to.x.atan2(to.z);
        let flat = (to.x * to.x + to.z * to.z).sqrt();
        let pitch = (-to.y).atan2(flat);
        Self {
            position: eye,
            yaw,
            pitch,
        }
    }

    fn forward(self) -> Vec3 {
        let cp = self.pitch.cos();
        Vec3::new(self.yaw.sin() * cp, self.pitch.sin(), self.yaw.cos() * cp)
    }
}

/// Held-key input for the flycam controller (look comes from the input session).
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

/// Elevated look-at for pre-spawn / lobby overview of the empty map (051).
pub fn overview_view_matrix() -> Mat4 {
    let eye = Vec3::new(0.0, 14.0, 18.0);
    let target = Vec3::ZERO;
    Mat4::look_at_rh(eye, target, Vec3::Y)
}

/// Camera: mounted = concepts [view] (look on the self); flycam is free, not view.
#[derive(Debug)]
pub struct ViewController {
    mode: ViewMode,
    fly: FlyPose,
    /// Last look position (eye socket) from present.
    mounted_eye: Vec3,
    /// Last look orientation (head forward) from present.
    mounted_forward: Vec3,
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
            fly: FlyPose::from_self(&SelfState::default_loadout(), DEFAULT_MOUNTED_EYE_M),
            mounted_eye: DEFAULT_MOUNTED_EYE_M,
            mounted_forward: DEFAULT_MOUNTED_FORWARD,
        }
    }

    /// Mount view on look (eye socket + head forward). Only FP orientation path.
    pub fn set_mounted_look(&mut self, eye: Vec3, forward: Vec3) {
        self.mounted_eye = eye;
        self.mounted_forward = if forward.length_squared() > 1e-12 {
            forward.normalize()
        } else {
            DEFAULT_MOUNTED_FORWARD
        };
    }

    /// World-space eye currently used for the mounted (FP) view.
    pub fn mounted_eye(&self) -> Vec3 {
        self.mounted_eye
    }

    pub fn is_flycam(&self) -> bool {
        self.mode == ViewMode::Flycam
    }

    /// Unmount at the given eye (must be the current FP camera position).
    pub fn enter_flycam(&mut self, self_state: &SelfState, eye: Vec3) {
        if self.mode == ViewMode::Mounted {
            self.mounted_eye = eye;
            self.fly = FlyPose::from_self(self_state, eye);
        }
        self.mode = ViewMode::Flycam;
    }

    /// Spectate free cam (overview loft start, not FP unmount).
    pub fn enter_spectate_flycam(&mut self) {
        if self.mode != ViewMode::Flycam {
            self.fly = FlyPose::overview_start();
        }
        self.mode = ViewMode::Flycam;
    }

    pub fn leave_flycam(&mut self, self_state: &SelfState) {
        self.mode = ViewMode::Mounted;
        self.fly = FlyPose::from_self(self_state, self.mounted_eye);
    }

    /// Sync mode from `cam.fly` after the mounted eye for this frame is known.
    /// Returns a status line if mode changed.
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

    pub fn update_flycam(&mut self, dt: f32, input: &FlyInput, look_px: glam::Vec2) {
        if self.mode != ViewMode::Flycam {
            return;
        }

        self.fly.yaw -= look_px.x * LOOK_SENS_RAD_PER_PX;
        self.fly.pitch = (self.fly.pitch - look_px.y * LOOK_SENS_RAD_PER_PX)
            .clamp(-LOOK_ELEV_CAP_RAD, LOOK_ELEV_CAP_RAD);

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

    pub fn view_matrix(&self) -> Mat4 {
        let (eye, forward) = self.eye_and_forward();
        let yaw = match self.mode {
            ViewMode::Mounted => forward.x.atan2(forward.z),
            ViewMode::Flycam => self.fly.yaw,
        };
        look_to_stable(eye, forward, yaw)
    }

    /// Mounted: last look from present. Flycam: free pose.
    pub fn eye_and_forward(&self) -> (Vec3, Vec3) {
        match self.mode {
            ViewMode::Mounted => (self.mounted_eye, self.mounted_forward),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_uses_set_mounted_look_not_drive_angles() {
        let mut v = ViewController::new();
        let (eye0, forward0) = v.eye_and_forward();
        assert!((eye0 - DEFAULT_MOUNTED_EYE_M).length() < 1e-5);
        assert!(forward0.dot(Vec3::Z) > 0.99);
        let mut s = SelfState::default_loadout();
        s.apply_look(1.0 / 60.0, 1.0, 0.5);
        let (_, still) = v.eye_and_forward();
        assert!(still.dot(Vec3::Z) > 0.99);
        let eye = Vec3::new(0.1, 1.5, 0.2);
        let fwd = Vec3::new(0.0, 0.0, 1.0);
        v.set_mounted_look(eye, fwd);
        let (e, f) = v.eye_and_forward();
        assert!((e - eye).length() < 1e-6);
        assert!(f.dot(Vec3::Z) > 0.99);
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
        let (_, forward) = v.eye_and_forward();
        assert!(forward.dot(Vec3::Z) > 0.99);
    }

    #[test]
    fn enter_flycam_matches_fp_eye_exactly() {
        let mut v = ViewController::new();
        let mut s = SelfState::default_loadout();
        s.apply_look(1.0 / 60.0, 0.4, -0.2);
        let fp_eye = Vec3::new(0.05, 1.44, 0.22);
        let fp_fwd = Vec3::new(0.2, -0.1, 0.97).normalize();
        v.set_mounted_look(fp_eye, fp_fwd);
        let (before_eye, before_fwd) = v.eye_and_forward();
        assert!((before_eye - fp_eye).length() < 1e-6);
        assert!((before_fwd - fp_fwd).length() < 1e-5);

        v.enter_flycam(&s, fp_eye);
        let (after_eye, _) = v.eye_and_forward();
        assert!(
            (after_eye - before_eye).length() < 1e-6,
            "fly eye jumped by {} m",
            (after_eye - before_eye).length()
        );
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
        let (eye, _) = v.eye_and_forward();
        assert!(eye.z > z0 + 1.0);
    }

    #[test]
    fn flycam_ws_follows_pitch() {
        let mut v = ViewController::new();
        let s = SelfState::default_loadout();
        v.enter_flycam(&s, v.mounted_eye());
        // Enough pitch that 1s at fly speed climbs >1 m (LOOK_SENS * dy).
        v.update_flycam(0.0, &FlyInput::default(), glam::Vec2::new(0.0, -1200.0));
        let y0 = v.eye_and_forward().0.y;
        v.update_flycam(
            1.0,
            &FlyInput {
                forward: true,
                ..Default::default()
            },
            glam::Vec2::ZERO,
        );
        let y1 = v.eye_and_forward().0.y;
        assert!(
            y1 > y0 + 1.0,
            "expected look-relative climb, y0={y0} y1={y1}"
        );
    }

    #[test]
    fn view_matrix_stable_at_straight_up() {
        let mut v = ViewController::new();
        v.set_mounted_look(DEFAULT_MOUNTED_EYE_M, Vec3::Y);
        let m = v.view_matrix();
        assert!(m.is_finite());
        let (_, forward) = v.eye_and_forward();
        assert!(forward.dot(Vec3::Y) > 0.99);
    }

    #[test]
    fn flycam_keeps_full_pitch_from_fp() {
        let mut v = ViewController::new();
        let mut s = SelfState::default_loadout();
        s.look_offset_pitch = -LOOK_ELEV_CAP_RAD;
        s.sync_pose();
        let eye = v.mounted_eye();
        v.enter_flycam(&s, eye);
        // Same clamp path as a free-look frame with no mouse delta.
        v.update_flycam(1.0 / 60.0, &FlyInput::default(), glam::Vec2::ZERO);
        let (_, fwd) = v.eye_and_forward();
        assert!(
            fwd.dot(-Vec3::Y) > 0.999,
            "flycam must keep straight-down from FP, fwd={fwd}"
        );
        assert!((v.fly.pitch + LOOK_ELEV_CAP_RAD).abs() < 1e-5);
    }

    #[test]
    fn spectate_starts_in_flycam() {
        let mut v = ViewController::new();
        v.enter_spectate_flycam();
        assert!(v.is_flycam());
        let (eye, _) = v.eye_and_forward();
        assert!(eye.y > 2.0);
    }
}
