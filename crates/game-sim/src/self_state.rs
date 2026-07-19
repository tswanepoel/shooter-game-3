//! Player self: position, look command, and body presentation pose.

use glam::{Mat4, Quat, Vec3};

/// Look elevation hard cap: straight up / straight down (015).
pub const OCULAR_ELEV_CAP_RAD: f32 = std::f32::consts::FRAC_PI_2;

/// Settled torso pitch at full look-down (inward).
pub const TORSO_PITCH_INWARD_RAD: f32 = 15.0_f32.to_radians();
/// Settled torso pitch at full look-up (outward).
pub const TORSO_PITCH_OUTWARD_RAD: f32 = 7.5_f32.to_radians();
/// Settled shoulder pitch at full look-down (inward).
pub const SHOULDER_PITCH_INWARD_RAD: f32 = 75.0_f32.to_radians();
/// Settled shoulder pitch at full look-up (outward).
pub const SHOULDER_PITCH_OUTWARD_RAD: f32 = 82.5_f32.to_radians();

const HEAD_PITCH_BUDGET_RAD: f32 = 50.0_f32.to_radians();

/// Default range along look for aim markers when max range is unknown (metres).
pub const DEFAULT_BORE_RANGE_M: f32 = 100.0;
/// World depth of the screen-centre reticle billboard (metres).
pub const RETICLE_DEPTH_M: f32 = 4.0;
/// On-screen reticle diameter (CSS/logical px).
pub const RETICLE_SIZE_PX: f32 = 6.0;

/// Head-local face offset in character-kit units (applied under posed `head` node).
/// With character-a head scale 0.1 and kit→m 1/1.5: rest eye ≈ (0, 1.43, 0.23) m.
pub const FACE_OFFSET_HEAD_KIT: Vec3 = Vec3::new(0.0, 2.5, 3.5);

#[derive(Debug, Clone, PartialEq)]
pub struct SelfState {
    pub position: Vec3,
    /// Look azimuth (radians). 0 faces **+Z**.
    pub ocular_yaw: f32,
    /// Look elevation (radians). Positive looks up. Clamped to ±90°.
    pub ocular_pitch: f32,
    pub character: u8,
    pub blaster: u8,
    pub alive: bool,
    pub armed: bool,

    /// Body absolute yaw (presentation; matches look in sim).
    pub torso_yaw: f32,
    pub torso_pitch: f32,
    pub shoulder_pitch: f32,
    /// Head relative yaw (cosmetic), body space.
    pub head_yaw: f32,
    /// Head relative pitch (cosmetic).
    pub head_pitch: f32,
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
        }
    }

    /// Unit look direction (yaw + pitch). Aim / camera forward.
    pub fn ocular_forward(&self) -> Vec3 {
        let cp = self.ocular_pitch.cos();
        Vec3::new(
            self.ocular_yaw.sin() * cp,
            self.ocular_pitch.sin(),
            self.ocular_yaw.cos() * cp,
        )
    }

    /// Body facing on XZ from torso yaw.
    pub fn body_facing(&self) -> Vec3 {
        Vec3::new(self.torso_yaw.sin(), 0.0, self.torso_yaw.cos())
    }

    /// Root placement: feet position + absolute body yaw.
    pub fn placement_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_y(self.torso_yaw), self.position)
    }

    /// Apply mouse look deltas (radians) and snap body presentation to look.
    /// `dt` is unused; kept so call sites stay frame-shaped.
    pub fn apply_look(&mut self, _dt: f32, delta_yaw: f32, delta_pitch: f32) {
        self.ocular_yaw += delta_yaw;
        self.ocular_pitch =
            (self.ocular_pitch + delta_pitch).clamp(-OCULAR_ELEV_CAP_RAD, OCULAR_ELEV_CAP_RAD);
        self.sync_pose();
    }

    /// Snap body presentation pose to current look (no lag in sim).
    pub fn sync_pose(&mut self) {
        let (torso_tgt, shoulder_tgt) = elevation_targets(self.ocular_pitch);
        self.torso_yaw = self.ocular_yaw;
        self.torso_pitch = torso_tgt;
        self.shoulder_pitch = shoulder_tgt;
        // Torso yaw matches look; head yaw relative is zero.
        self.head_yaw = 0.0;
        self.head_pitch = (self.ocular_pitch - self.torso_pitch)
            .clamp(-HEAD_PITCH_BUDGET_RAD, HEAD_PITCH_BUDGET_RAD);
    }

    /// World point for the screen-centre reticle billboard (along look from `eye`).
    pub fn reticle_world(&self, eye: Vec3) -> Option<Vec3> {
        if !(self.alive && self.armed) {
            return None;
        }
        let dir = self.ocular_forward();
        if dir.length_squared() < 1e-12 {
            return None;
        }
        Some(eye + dir * RETICLE_DEPTH_M)
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
    fn look_snaps_body_pose_immediately() {
        let mut s = SelfState::default_loadout();
        s.apply_look(1.0 / 60.0, 1.0, 0.3);
        assert!((s.torso_yaw - s.ocular_yaw).abs() < 1e-6);
        assert!((s.head_yaw).abs() < 1e-6);
        let (torso_tgt, shoulder_tgt) = elevation_targets(s.ocular_pitch);
        assert!((s.torso_pitch - torso_tgt).abs() < 1e-6);
        assert!((s.shoulder_pitch - shoulder_tgt).abs() < 1e-6);
    }

    #[test]
    fn elevation_at_full_look_up() {
        let mut s = SelfState::default_loadout();
        s.ocular_pitch = OCULAR_ELEV_CAP_RAD;
        s.sync_pose();
        assert!((s.torso_pitch - TORSO_PITCH_OUTWARD_RAD).abs() < 1e-5);
        assert!((s.shoulder_pitch - SHOULDER_PITCH_OUTWARD_RAD).abs() < 1e-5);
        let f = s.ocular_forward();
        assert!(f.dot(Vec3::Y) > 0.99, "forward={f}");
    }

    #[test]
    fn ocular_pitch_clamped_to_pm_90() {
        let mut s = SelfState::default_loadout();
        s.apply_look(1.0 / 60.0, 0.0, 10.0);
        assert!((s.ocular_pitch - OCULAR_ELEV_CAP_RAD).abs() < 1e-5);
        s.apply_look(1.0 / 60.0, 0.0, -20.0);
        assert!((s.ocular_pitch + OCULAR_ELEV_CAP_RAD).abs() < 1e-5);
    }

    #[test]
    fn reticle_lies_on_look_ray() {
        let s = SelfState::default_loadout();
        let eye = Vec3::new(0.0, 1.5, 0.0);
        let r = s.reticle_world(eye).expect("armed");
        let along = (r - eye).normalize();
        assert!(along.dot(s.ocular_forward()) > 0.99);
        assert!(((r - eye).length() - RETICLE_DEPTH_M).abs() < 1e-5);
    }
}
