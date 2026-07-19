//! Player self: position, look, walk drive, and body presentation pose.

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

/// Walk speed on the ground plane (016).
/// Kenney `walk` at 1×: stance sole slip = \(2 L \sin\theta\) per half-cycle
/// (\(L = 2/3\,\mathrm{m}\), \(\theta = 60°\), \(T = 2/3\,\mathrm{s}\)) → \(2\sqrt{3}\) m/s.
pub const WALK_SPEED_M_S: f32 = 3.464_101_6; // 2√3
/// Kenney `walk` clip duration (seconds). Phase maps as `phase * duration`.
pub const WALK_CLIP_DURATION_S: f32 = 2.0 / 3.0;
/// Ground metres per full walk cycle (phase 0→1). At walk speed this plays the clip at 1×.
pub const WALK_STRIDE_M: f32 = WALK_SPEED_M_S * WALK_CLIP_DURATION_S;

/// Head-local face offset in character-kit units (applied under posed `head` node).
/// With character-a head scale 0.1 and kit→m 1/1.5: rest eye ≈ (0, 1.43, 0.23) m.
pub const FACE_OFFSET_HEAD_KIT: Vec3 = Vec3::new(0.0, 2.5, 3.5);

/// Ground locomotion mode (016).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocomotionMode {
    #[default]
    Stand,
    Walk,
}

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

    /// Look-relative forward wish (−1…1). Positive is W.
    pub wish_forward: f32,
    /// Look-relative strafe wish (−1…1). Positive is D (right).
    pub wish_strafe: f32,
    pub locomotion: LocomotionMode,
    /// Fraction through the walk cycle, [0, 1).
    pub walk_phase: f32,

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
            wish_forward: 0.0,
            wish_strafe: 0.0,
            locomotion: LocomotionMode::Stand,
            walk_phase: 0.0,
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

    /// Horizontal look forward on XZ (unit, y = 0).
    pub fn look_forward_xz(&self) -> Vec3 {
        Vec3::new(self.ocular_yaw.sin(), 0.0, self.ocular_yaw.cos())
    }

    /// Horizontal look right on XZ (unit, y = 0).
    /// Matches RH view / flycam: `forward_xz × world_up` (screen-right).
    pub fn look_right_xz(&self) -> Vec3 {
        let f = self.look_forward_xz();
        Vec3::new(-f.z, 0.0, f.x)
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

    /// Apply look-relative walk wish and integrate ground position (016).
    ///
    /// `forward` / `strafe` are digital axes (−1…1). Diagonals normalize. Speed is
    /// constant [`WALK_SPEED_M_S`]. Phase advances with distance over [`WALK_STRIDE_M`].
    pub fn apply_move(&mut self, dt: f32, forward: f32, strafe: f32) {
        self.wish_forward = forward.clamp(-1.0, 1.0);
        self.wish_strafe = strafe.clamp(-1.0, 1.0);

        let mut wish =
            self.look_forward_xz() * self.wish_forward + self.look_right_xz() * self.wish_strafe;
        wish.y = 0.0;

        if wish.length_squared() > 1e-12 {
            let dir = wish.normalize();
            let step = WALK_SPEED_M_S * dt.max(0.0);
            self.position += dir * step;
            self.position.y = 0.0;
            self.locomotion = LocomotionMode::Walk;
            if WALK_STRIDE_M > 1e-8 {
                self.walk_phase = (self.walk_phase + step / WALK_STRIDE_M).rem_euclid(1.0);
            }
        } else {
            self.locomotion = LocomotionMode::Stand;
        }

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
        assert_eq!(s.locomotion, LocomotionMode::Stand);
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

    #[test]
    fn walk_forward_along_look_at_constant_speed() {
        let mut s = SelfState::default_loadout();
        s.apply_move(1.0, 1.0, 0.0);
        assert_eq!(s.locomotion, LocomotionMode::Walk);
        assert!((s.position.z - WALK_SPEED_M_S).abs() < 1e-5);
        assert!(s.position.x.abs() < 1e-5);
        assert!(s.position.y.abs() < 1e-5);
        let expect_phase = (WALK_SPEED_M_S / WALK_STRIDE_M).rem_euclid(1.0);
        assert!((s.walk_phase - expect_phase).abs() < 1e-5);
    }

    #[test]
    fn diagonal_wish_normalizes_speed() {
        let mut s = SelfState::default_loadout();
        s.apply_move(1.0, 1.0, 1.0);
        let dist = s.position.length();
        assert!((dist - WALK_SPEED_M_S).abs() < 1e-4, "dist={dist}");
    }

    #[test]
    fn strafe_is_look_relative_and_keys_do_not_yaw() {
        let mut s = SelfState::default_loadout();
        s.ocular_yaw = 0.0;
        s.apply_move(1.0, 0.0, 1.0);
        // Facing +Z, screen-right is −X (RH look_to / forward × up).
        assert!((s.position.x + WALK_SPEED_M_S).abs() < 1e-4);
        assert!(s.position.z.abs() < 1e-4);
        assert!((s.torso_yaw - s.ocular_yaw).abs() < 1e-6);
        assert_eq!(s.ocular_yaw, 0.0);
    }

    #[test]
    fn zero_wish_stands() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.1, 1.0, 0.0);
        let phase = s.walk_phase;
        s.apply_move(0.1, 0.0, 0.0);
        assert_eq!(s.locomotion, LocomotionMode::Stand);
        assert!((s.walk_phase - phase).abs() < 1e-6);
    }
}
