//! Player self: position, ocular command, derived aim cascade.

use glam::{Mat4, Quat, Vec3};

/// Ocular elevation hard cap. Weapon budget may exceed this.
pub const OCULAR_ELEV_CAP_RAD: f32 = 80.0_f32.to_radians();

/// Settled torso pitch at full look-down (inward).
pub const TORSO_PITCH_INWARD_RAD: f32 = 15.0_f32.to_radians();
/// Settled torso pitch at full look-up (outward).
pub const TORSO_PITCH_OUTWARD_RAD: f32 = 7.5_f32.to_radians();
/// Settled shoulder pitch at full look-down (inward).
pub const SHOULDER_PITCH_INWARD_RAD: f32 = 75.0_f32.to_radians();
/// Settled shoulder pitch at full look-up (outward).
pub const SHOULDER_PITCH_OUTWARD_RAD: f32 = 82.5_f32.to_radians();

const HEAD_PITCH_BUDGET_RAD: f32 = 50.0_f32.to_radians();
const HEAD_YAW_BUDGET_RAD: f32 = 60.0_f32.to_radians();

const RATE_HEAD_SNAPPY: f32 = 368.0;
const RATE_HEAD_LAGGY: f32 = 96.0;
const RATE_TORSO_YAW_SNAPPY: f32 = 256.0;
const RATE_TORSO_YAW_LAGGY: f32 = 32.0;
const RATE_PITCH_SNAPPY: f32 = 48.0;
const RATE_PITCH_LAGGY: f32 = 10.0;
const LOOK_SPEED_SMOOTH: f32 = 4.0;
/// Look-speed (rad/s) at which chase rates fully soften to laggy.
const LOOK_SPEED_SOFT_RAD_S: f32 = 8.0;

/// Default bore ray length when blaster max range is unknown (metres).
pub const DEFAULT_BORE_RANGE_M: f32 = 100.0;
/// Reticle nudge toward camera (metres).
pub const RETICLE_CAM_NUDGE_M: f32 = 0.03;
/// On-screen reticle diameter (CSS/logical px).
pub const RETICLE_SIZE_PX: f32 = 6.0;

/// Head-local face offset in character-kit units (applied under posed `head` node).
/// Chosen so the eye sits in the face volume at rest (~1.52 m height, slight +Z).
pub const FACE_OFFSET_HEAD_KIT: Vec3 = Vec3::new(0.0, 2.5, 3.5);

#[derive(Debug, Clone, PartialEq)]
pub struct SelfState {
    pub position: Vec3,
    /// Ocular azimuth (radians). 0 faces **+Z**.
    pub ocular_yaw: f32,
    /// Ocular elevation (radians). Positive looks up.
    pub ocular_pitch: f32,
    pub character: u8,
    pub blaster: u8,
    pub alive: bool,
    pub armed: bool,

    /// Body absolute yaw (lags ocular).
    pub torso_yaw: f32,
    pub torso_pitch: f32,
    pub shoulder_pitch: f32,
    /// Head relative yaw (cosmetic), body space.
    pub head_yaw: f32,
    /// Head relative pitch (cosmetic).
    pub head_pitch: f32,

    look_speed_smooth: f32,
}

impl Default for SelfState {
    fn default() -> Self {
        Self::default_loadout()
    }
}

impl SelfState {
    pub fn default_loadout() -> Self {
        Self {
            position: Vec3::ZERO,
            ocular_yaw: 0.0,
            ocular_pitch: 0.0,
            character: b'a',
            blaster: b'p',
            alive: true,
            armed: true,
            torso_yaw: 0.0,
            torso_pitch: 0.0,
            shoulder_pitch: 0.0,
            head_yaw: 0.0,
            head_pitch: 0.0,
            look_speed_smooth: 0.0,
        }
    }

    /// Unit ocular look direction (yaw + pitch).
    pub fn ocular_forward(&self) -> Vec3 {
        let cp = self.ocular_pitch.cos();
        Vec3::new(
            self.ocular_yaw.sin() * cp,
            self.ocular_pitch.sin(),
            self.ocular_yaw.cos() * cp,
        )
    }

    /// Body facing on XZ from lagged torso yaw.
    pub fn body_facing(&self) -> Vec3 {
        Vec3::new(self.torso_yaw.sin(), 0.0, self.torso_yaw.cos())
    }

    /// Root placement: feet position + absolute body yaw.
    pub fn placement_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_y(self.torso_yaw), self.position)
    }

    /// Apply mouse look deltas (radians) then step the aim cascade.
    pub fn apply_look(&mut self, dt: f32, delta_yaw: f32, delta_pitch: f32) {
        let look_speed = if dt > 1e-6 {
            (delta_yaw * delta_yaw + delta_pitch * delta_pitch).sqrt() / dt
        } else {
            0.0
        };
        self.look_speed_smooth +=
            (look_speed - self.look_speed_smooth) * (1.0 - (-LOOK_SPEED_SMOOTH * dt).exp());

        self.ocular_yaw += delta_yaw;
        self.ocular_pitch =
            (self.ocular_pitch + delta_pitch).clamp(-OCULAR_ELEV_CAP_RAD, OCULAR_ELEV_CAP_RAD);

        self.step_cascade(dt);
    }

    /// Step cascade without new look input (settle).
    pub fn step_cascade(&mut self, dt: f32) {
        let soft = (self.look_speed_smooth / LOOK_SPEED_SOFT_RAD_S).clamp(0.0, 1.0);
        // Fast look → laggy rates; slow/held → snappy.
        let rate = |snappy: f32, laggy: f32| laggy + (snappy - laggy) * (1.0 - soft);

        let (torso_tgt, shoulder_tgt) = elevation_targets(self.ocular_pitch);
        let head_yaw_tgt =
            (self.ocular_yaw - self.torso_yaw).clamp(-HEAD_YAW_BUDGET_RAD, HEAD_YAW_BUDGET_RAD);
        let head_pitch_tgt = (self.ocular_pitch - self.torso_pitch)
            .clamp(-HEAD_PITCH_BUDGET_RAD, HEAD_PITCH_BUDGET_RAD);

        let r_ty = rate(RATE_TORSO_YAW_SNAPPY, RATE_TORSO_YAW_LAGGY);
        let r_pitch = rate(RATE_PITCH_SNAPPY, RATE_PITCH_LAGGY);
        let r_head = rate(RATE_HEAD_SNAPPY, RATE_HEAD_LAGGY);

        self.torso_yaw = exp_chase_angle(self.torso_yaw, self.ocular_yaw, r_ty, dt);
        self.torso_pitch = exp_chase(self.torso_pitch, torso_tgt, r_pitch, dt);
        self.shoulder_pitch = exp_chase(self.shoulder_pitch, shoulder_tgt, r_pitch, dt);
        self.head_yaw = exp_chase(self.head_yaw, head_yaw_tgt, r_head, dt);
        self.head_pitch = exp_chase(self.head_pitch, head_pitch_tgt, r_head, dt);
    }

    /// Weapon elevation relative to ocular (settled ≈ torso + shoulder − ocular).
    pub fn weapon_elev_separation(&self) -> f32 {
        self.torso_pitch + self.shoulder_pitch - self.ocular_pitch
    }

    pub fn weapon_azim_separation(&self) -> f32 {
        angle_delta(self.torso_yaw, self.ocular_yaw)
    }
}

fn elevation_targets(ocular_pitch: f32) -> (f32, f32) {
    let t = (ocular_pitch / OCULAR_ELEV_CAP_RAD).clamp(-1.0, 1.0);
    if t >= 0.0 {
        (t * TORSO_PITCH_OUTWARD_RAD, t * SHOULDER_PITCH_OUTWARD_RAD)
    } else {
        (t * TORSO_PITCH_INWARD_RAD, t * SHOULDER_PITCH_INWARD_RAD)
    }
}

fn exp_chase(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    current + (target - current) * (1.0 - (-rate * dt).exp())
}

fn exp_chase_angle(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    let d = angle_delta(current, target);
    current + d * (1.0 - (-rate * dt).exp())
}

fn angle_delta(from: f32, to: f32) -> f32 {
    let mut d = to - from;
    let pi = std::f32::consts::PI;
    while d > pi {
        d -= 2.0 * pi;
    }
    while d < -pi {
        d += 2.0 * pi;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_faces_plus_z_at_origin() {
        let s = SelfState::default_loadout();
        assert_eq!(s.position, Vec3::ZERO);
        assert_eq!(s.character, b'a');
        assert_eq!(s.blaster, b'p');
        let f = s.ocular_forward();
        assert!(f.dot(Vec3::Z) > 0.99);
        assert!((f.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn azimuth_settles_to_ocular() {
        let mut s = SelfState::default_loadout();
        s.ocular_yaw = 1.0;
        for _ in 0..120 {
            s.step_cascade(1.0 / 60.0);
        }
        assert!(
            s.weapon_azim_separation().abs() < 0.02,
            "sep={}",
            s.weapon_azim_separation()
        );
    }

    #[test]
    fn elevation_budget_at_full_look_up() {
        let mut s = SelfState::default_loadout();
        s.ocular_pitch = OCULAR_ELEV_CAP_RAD;
        for _ in 0..180 {
            s.step_cascade(1.0 / 60.0);
        }
        let weapon = s.torso_pitch + s.shoulder_pitch;
        assert!(
            (weapon - (TORSO_PITCH_OUTWARD_RAD + SHOULDER_PITCH_OUTWARD_RAD)).abs() < 0.05,
            "weapon elev={weapon}"
        );
        assert!(s.weapon_elev_separation() > 0.1);
    }

    #[test]
    fn ocular_pitch_clamped() {
        let mut s = SelfState::default_loadout();
        s.apply_look(1.0 / 60.0, 0.0, 10.0);
        assert!((s.ocular_pitch - OCULAR_ELEV_CAP_RAD).abs() < 1e-5);
    }
}
