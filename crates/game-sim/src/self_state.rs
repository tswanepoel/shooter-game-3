//! Player self: position, look, walk drive, and look-synced body joints.
//!
//! Presentation builds look pose (mount/aim) and present pose (drawn body) from this drive (017).

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

/// Sprint speed on the ground plane (020). ~1.75× walk.
pub const SPRINT_SPEED_M_S: f32 = WALK_SPEED_M_S * 1.75;
/// Kenney `sprint` clip duration (seconds).
pub const SPRINT_CLIP_DURATION_S: f32 = 0.5;
/// Ground metres per full sprint cycle (phase 0→1). At sprint speed this plays the clip at 1×.
pub const SPRINT_STRIDE_M: f32 = SPRINT_SPEED_M_S * SPRINT_CLIP_DURATION_S;

/// Full stamina (0…1).
pub const STAMINA_MAX: f32 = 1.0;
/// Continuous sprint duration on a full bar (seconds).
pub const STAMINA_SPRINT_S: f32 = 4.0;
/// Full refill time while not sprinting (seconds).
pub const STAMINA_REGEN_S: f32 = 4.0;
/// Minimum fill required to *start* a sprint (avoid premature flicker).
pub const STAMINA_MIN_TO_START: f32 = 0.25;

pub const JUMP_PEAK_M: f32 = 1.2;
pub const JUMP_TIME_TO_APEX_S: f32 = 0.25;
pub const JUMP_GRAVITY_M_S2: f32 = 2.0 * JUMP_PEAK_M / (JUMP_TIME_TO_APEX_S * JUMP_TIME_TO_APEX_S);
pub const JUMP_LAUNCH_M_S: f32 = JUMP_GRAVITY_M_S2 * JUMP_TIME_TO_APEX_S;

/// Head-local face offset in character-kit units (applied under posed `head` node).
///
/// Chosen so rest look origin matches the calibrated feet-local eye
/// `(0, 1.52, 0.27)` m (character-a: head pivot `(0, 1.9, 0)` kit, scale `0.1`,
/// kit→m `1/1.5`, soles on y = 0).
pub const FACE_OFFSET_HEAD_KIT: Vec3 = Vec3::new(0.0, 3.8, 4.05);

/// Locomotion mode. `Stopping` = in-place walk settle to neutral, then [`Stand`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocomotionMode {
    #[default]
    Stand,
    Walk,
    Sprint,
    Stopping,
    Air,
}

impl LocomotionMode {
    pub fn uses_walk_clip(self) -> bool {
        matches!(self, Self::Walk | Self::Stopping)
    }

    pub fn uses_loco_clip(self) -> bool {
        matches!(self, Self::Walk | Self::Sprint | Self::Stopping)
    }

    pub fn is_sprint(self) -> bool {
        matches!(self, Self::Sprint)
    }

    pub fn is_air(self) -> bool {
        matches!(self, Self::Air)
    }
}

/// Blaster class (021). Secondary may only hold launcher or pistol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponClass {
    Launcher,
    Pistol,
    Smg,
    AssaultRifle,
    SniperRifle,
    Shotgun,
}

impl WeaponClass {
    pub fn from_letter(letter: u8) -> Option<Self> {
        Some(match letter {
            b'a' => Self::Launcher,
            b'b' | b'i' => Self::Pistol,
            b'c' | b'g' | b'h' | b'l' | b'm' | b'p' => Self::Smg,
            b'd' | b'n' | b'q' | b'r' => Self::AssaultRifle,
            b'e' | b'f' => Self::SniperRifle,
            b'j' | b'k' | b'o' => Self::Shotgun,
            _ => return None,
        })
    }

    pub fn allowed_in_secondary(self) -> bool {
        matches!(self, Self::Launcher | Self::Pistol)
    }
}

/// Which loadout slot is in hand (021). Unarmed = active slot empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveWeapon {
    #[default]
    Primary,
    Secondary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfState {
    pub position: Vec3,
    /// Look azimuth (radians). 0 faces **+Z**.
    pub ocular_yaw: f32,
    /// Look elevation (radians). Positive looks up. Clamped to ±90°.
    pub ocular_pitch: f32,
    pub character: u8,
    /// Primary slot blaster letter (`a`…`r`), or empty (021).
    pub primary: Option<u8>,
    /// Secondary slot blaster letter; launcher/pistol only when set (021).
    pub secondary: Option<u8>,
    /// Which hand is active: a filled slot or unarmed (021).
    pub active: ActiveWeapon,
    pub alive: bool,

    /// Look-relative forward wish (−1…1). Positive is W.
    pub wish_forward: f32,
    /// Look-relative strafe wish (−1…1). Positive is D (right).
    pub wish_strafe: f32,
    pub locomotion: LocomotionMode,
    /// Fraction through the locomotion cycle, [0, 1).
    pub walk_phase: f32,
    /// Sprint stamina 0…[`STAMINA_MAX`] (020).
    pub stamina: f32,
    /// Sticky sprint wish from Shift tap (020); cleared on cancel, stop, or empty bar.
    pub sprint_latched: bool,
    pub velocity_y: f32,
    /// Horizontal air velocity locked at jump (world XZ).
    pub air_vel_x: f32,
    pub air_vel_z: f32,

    /// Body absolute yaw (presentation; matches look in sim).
    pub torso_yaw: f32,
    pub torso_pitch: f32,
    pub shoulder_pitch: f32,
    /// Head relative yaw (cosmetic), body space.
    pub head_yaw: f32,
    /// Head relative pitch (cosmetic).
    pub head_pitch: f32,

    /// Active emote wheel slot (`0`…`3`), if any (039).
    pub emote: Option<u8>,
    /// Seconds since emote commit (039). Cleared with [`Self::clear_emote`].
    pub emote_age_s: f32,
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
            primary: Some(b'p'),
            secondary: Some(b'b'),
            active: ActiveWeapon::Primary,
            alive: true,
            wish_forward: 0.0,
            wish_strafe: 0.0,
            locomotion: LocomotionMode::Stand,
            walk_phase: 0.0,
            stamina: STAMINA_MAX,
            sprint_latched: false,
            velocity_y: 0.0,
            air_vel_x: 0.0,
            air_vel_z: 0.0,
            torso_yaw: 0.0,
            torso_pitch: 0.0,
            shoulder_pitch: 0.0,
            head_yaw: 0.0,
            head_pitch: 0.0,
            emote: None,
            emote_age_s: 0.0,
        }
    }

    /// Letter of the active slot, if that slot is filled.
    pub fn active_blaster(&self) -> Option<u8> {
        match self.active {
            ActiveWeapon::Primary => self.primary,
            ActiveWeapon::Secondary => self.secondary,
        }
    }

    /// True when the active slot holds a blaster.
    pub fn is_armed(&self) -> bool {
        self.active_blaster().is_some()
    }

    /// True while an emote clip is playing (039).
    pub fn is_emoting(&self) -> bool {
        self.emote.is_some()
    }

    /// Holster present: emote owns arms; do not draw hold/blaster (039 policy A).
    /// Loadout identity is unchanged.
    pub fn emote_holster(&self) -> bool {
        self.is_emoting()
    }

    /// Present-armed: active letter filled and not holstered for emote.
    pub fn presents_armed(&self) -> bool {
        self.is_armed() && !self.emote_holster()
    }

    /// Clear emote drive (natural end, cancel, replace prep).
    pub fn clear_emote(&mut self) {
        self.emote = None;
        self.emote_age_s = 0.0;
    }

    /// Advance emote age; clear when the kit clip duration elapses.
    pub fn tick_emote(&mut self, dt: f32) {
        let Some(id) = self.emote else {
            return;
        };
        self.emote_age_s += dt.max(0.0);
        let dur = crate::emote_duration_s(id);
        if dur <= 0.0 || self.emote_age_s >= dur {
            self.clear_emote();
        }
    }

    /// Commit a wheel slot. Requires grounded; `weapon_side_blocked` is burst (038).
    /// Replaces an in-flight emote. Clears sprint latch.
    pub fn try_commit_emote(&mut self, id: u8, weapon_side_blocked: bool) -> bool {
        if weapon_side_blocked || !self.is_grounded() {
            return false;
        }
        if crate::emote_def(id).is_none() {
            return false;
        }
        self.sprint_latched = false;
        self.emote = Some(id);
        self.emote_age_s = 0.0;
        true
    }

    /// Set primary (any class, or clear). Invalid letter rejected.
    pub fn set_primary(&mut self, letter: Option<u8>) -> Result<(), &'static str> {
        if let Some(l) = letter {
            WeaponClass::from_letter(l).ok_or("unknown blaster letter")?;
        }
        self.primary = letter;
        Ok(())
    }

    /// Set secondary (launcher/pistol only, or clear). Invalid class rejected.
    pub fn set_secondary(&mut self, letter: Option<u8>) -> Result<(), &'static str> {
        if let Some(l) = letter {
            let class = WeaponClass::from_letter(l).ok_or("unknown blaster letter")?;
            if !class.allowed_in_secondary() {
                return Err("secondary only allows launcher or pistol");
            }
        }
        self.secondary = letter;
        Ok(())
    }

    /// Toggle active slot: primary ↔ secondary. Empty slots stay in the cycle (unarmed).
    /// Cancels emote (039) so the new hand is free immediately.
    pub fn cycle_weapon(&mut self, dir: i8) {
        if dir.signum() == 0 {
            return;
        }
        self.clear_emote();
        self.active = match self.active {
            ActiveWeapon::Primary => ActiveWeapon::Secondary,
            ActiveWeapon::Secondary => ActiveWeapon::Primary,
        };
    }

    pub fn is_grounded(&self) -> bool {
        !self.locomotion.is_air() && self.position.y <= 1e-5 && self.velocity_y <= 0.0
    }

    /// Ground plane speed for the current locomotion (walk or sprint).
    pub fn ground_speed(&self) -> f32 {
        if self.locomotion.is_sprint() {
            SPRINT_SPEED_M_S
        } else {
            WALK_SPEED_M_S
        }
    }

    pub fn try_jump(&mut self) {
        if !self.is_grounded() {
            return;
        }
        self.clear_emote();
        let speed = self.ground_speed();
        let mut wish =
            self.look_forward_xz() * self.wish_forward + self.look_right_xz() * self.wish_strafe;
        wish.y = 0.0;
        if wish.length_squared() > 1e-12 {
            let dir = wish.normalize();
            self.air_vel_x = dir.x * speed;
            self.air_vel_z = dir.z * speed;
        } else {
            self.air_vel_x = 0.0;
            self.air_vel_z = 0.0;
        }
        self.velocity_y = JUMP_LAUNCH_M_S;
        self.locomotion = LocomotionMode::Air;
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

    /// Set absolute look (radians) and snap body presentation. Used by server net apply.
    pub fn set_look(&mut self, yaw: f32, pitch: f32) {
        self.ocular_yaw = yaw;
        self.ocular_pitch = pitch.clamp(-OCULAR_ELEV_CAP_RAD, OCULAR_ELEV_CAP_RAD);
        self.sync_pose();
    }

    /// Look-relative walk wish (−1…1). Ground: phase from distance. Air: coast + gravity.
    /// `sprint_tap` is a Shift press edge (020); latches sprint only (no cancel). Stamina gates start/drain.
    pub fn apply_move(&mut self, dt: f32, forward: f32, strafe: f32, sprint_tap: bool) {
        self.wish_forward = forward.clamp(-1.0, 1.0);
        self.wish_strafe = strafe.clamp(-1.0, 1.0);

        // Emote cancels on any walk wish or sprint engage (039).
        let wish_moving = self.wish_forward.abs() > 1e-6 || self.wish_strafe.abs() > 1e-6;
        if self.is_emoting() && (wish_moving || sprint_tap) {
            self.clear_emote();
        }

        // Shift tap only engages; never cancels (empty bar / stop / lose W clear the latch).
        if sprint_tap && !self.sprint_latched && self.stamina >= STAMINA_MIN_TO_START {
            self.sprint_latched = true;
        }

        let mut wish =
            self.look_forward_xz() * self.wish_forward + self.look_right_xz() * self.wish_strafe;
        wish.y = 0.0;

        let dt = dt.max(0.0);
        let moving = wish.length_squared() > 1e-12;

        if self.locomotion.is_air() {
            self.integrate_air(dt, moving);
            // Latched sprint still costs stamina aloft (no jump-regen exploit).
            if self.sprint_latched {
                self.drain_stamina(dt);
            } else {
                self.regen_stamina(dt);
            }
        } else if moving {
            // Sprint is forward-only (W); A/D/S alone walk.
            let forward_ok = self.wish_forward > 1e-6;
            if self.sprint_latched && !forward_ok {
                self.sprint_latched = false;
            }
            let sprinting = self.sprint_latched && forward_ok && self.stamina > 0.0;

            let (speed, stride) = if sprinting {
                (SPRINT_SPEED_M_S, SPRINT_STRIDE_M)
            } else {
                (WALK_SPEED_M_S, WALK_STRIDE_M)
            };

            let dir = wish.normalize();
            self.position += dir * (speed * dt);
            self.position.y = 0.0;
            self.locomotion = if sprinting {
                LocomotionMode::Sprint
            } else {
                LocomotionMode::Walk
            };
            let dphase = if stride > 1e-8 {
                speed * dt / stride
            } else {
                0.0
            };
            self.walk_phase = (self.walk_phase + dphase).rem_euclid(1.0);

            if self.sprint_latched {
                self.drain_stamina(dt);
                if !self.sprint_latched {
                    self.locomotion = LocomotionMode::Walk;
                }
            } else {
                self.regen_stamina(dt);
            }
        } else {
            self.sprint_latched = false;
            if self.locomotion.is_sprint() {
                self.locomotion = LocomotionMode::Stand;
                self.walk_phase = 0.0;
            } else {
                let dphase = if WALK_STRIDE_M > 1e-8 {
                    WALK_SPEED_M_S * dt / WALK_STRIDE_M
                } else {
                    0.0
                };
                self.settle_walk_stop(dphase);
            }
            self.regen_stamina(dt);
        }

        self.sync_pose();
    }

    fn drain_stamina(&mut self, dt: f32) {
        self.stamina = (self.stamina - dt / STAMINA_SPRINT_S).max(0.0);
        if self.stamina <= 0.0 {
            self.stamina = 0.0;
            self.sprint_latched = false;
        }
    }

    fn regen_stamina(&mut self, dt: f32) {
        if self.stamina < STAMINA_MAX {
            self.stamina = (self.stamina + dt / STAMINA_REGEN_S).min(STAMINA_MAX);
        }
    }

    fn integrate_air(&mut self, dt: f32, land_wish: bool) {
        self.position.x += self.air_vel_x * dt;
        self.position.z += self.air_vel_z * dt;

        self.velocity_y -= JUMP_GRAVITY_M_S2 * dt;
        self.position.y += self.velocity_y * dt;
        self.locomotion = LocomotionMode::Air;

        if self.position.y <= 0.0 && self.velocity_y <= 0.0 {
            self.position.y = 0.0;
            self.velocity_y = 0.0;
            self.air_vel_x = 0.0;
            self.air_vel_z = 0.0;
            self.walk_phase = 0.0;
            self.locomotion = if land_wish {
                LocomotionMode::Walk
            } else {
                LocomotionMode::Stand
            };
        }
    }

    /// Finish walk in place to the nearest neutral, then stand.
    ///
    /// Kenney walk neutrals land at phase 0 and 0.5 (same rest). From the first
    /// half, 0.5 is the fastest back-out; from the second half, the cycle end.
    fn settle_walk_stop(&mut self, dphase: f32) {
        let settling = matches!(
            self.locomotion,
            LocomotionMode::Walk | LocomotionMode::Stopping
        ) && self.walk_phase > 1e-6
            && (self.walk_phase - 0.5).abs() > 1e-6;

        if !settling {
            self.locomotion = LocomotionMode::Stand;
            self.walk_phase = 0.0;
            return;
        }

        // Target: mid-cycle neutral if still in first half, else cycle end.
        let target = if self.walk_phase < 0.5 { 0.5 } else { 1.0 };
        let next = self.walk_phase + dphase;
        if next >= target - 1e-6 {
            self.locomotion = LocomotionMode::Stand;
            self.walk_phase = 0.0;
        } else {
            self.locomotion = LocomotionMode::Stopping;
            self.walk_phase = next;
        }
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

    /// World point for the screen-centre reticle billboard (along look from look origin).
    pub fn reticle_world(&self, look_origin: Vec3) -> Option<Vec3> {
        if !(self.alive && self.presents_armed()) {
            return None;
        }
        let dir = self.ocular_forward();
        if dir.length_squared() < 1e-12 {
            return None;
        }
        Some(look_origin + dir * RETICLE_DEPTH_M)
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
        assert_eq!(s.primary, Some(b'p'));
        assert_eq!(s.secondary, Some(b'b'));
        assert_eq!(s.active, ActiveWeapon::Primary);
        assert_eq!(s.active_blaster(), Some(b'p'));
        assert!(s.is_armed());
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
        s.apply_move(1.0, 1.0, 0.0, false);
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
        s.apply_move(1.0, 1.0, 1.0, false);
        let dist = s.position.length();
        assert!((dist - WALK_SPEED_M_S).abs() < 1e-4, "dist={dist}");
    }

    #[test]
    fn strafe_is_look_relative_and_keys_do_not_yaw() {
        let mut s = SelfState::default_loadout();
        s.ocular_yaw = 0.0;
        s.apply_move(1.0, 0.0, 1.0, false);
        // Facing +Z, screen-right is −X (RH look_to / forward × up).
        assert!((s.position.x + WALK_SPEED_M_S).abs() < 1e-4);
        assert!(s.position.z.abs() < 1e-4);
        assert!((s.torso_yaw - s.ocular_yaw).abs() < 1e-6);
        assert_eq!(s.ocular_yaw, 0.0);
    }

    #[test]
    fn zero_wish_settles_to_nearest_neutral_then_stands() {
        let mut s = SelfState::default_loadout();
        // First half: stop should aim at mid-cycle neutral (0.5), not full end.
        s.apply_move(0.1, 1.0, 0.0, false);
        assert!(
            s.walk_phase > 1e-6 && s.walk_phase < 0.5,
            "phase={}",
            s.walk_phase
        );
        let pos = s.position;

        s.apply_move(1e-3, 0.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Stopping);
        assert!((s.position - pos).length() < 1e-6, "feet plant on stop");

        let remain = 0.5 - s.walk_phase;
        let dt_finish = remain * WALK_STRIDE_M / WALK_SPEED_M_S + 1e-3;
        s.apply_move(dt_finish, 0.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Stand);
        assert!((s.walk_phase).abs() < 1e-6);
        assert!((s.position - pos).length() < 1e-6);
    }

    #[test]
    fn stop_in_second_half_settles_to_cycle_end() {
        let mut s = SelfState::default_loadout();
        // One full stride-second lands past mid (speed/stride * t).
        s.apply_move(0.4, 1.0, 0.0, false);
        assert!(s.walk_phase >= 0.5, "phase={}", s.walk_phase);
        let pos = s.position;
        let remain = 1.0 - s.walk_phase;
        s.apply_move(1e-3, 0.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Stopping);
        let dt_finish = (remain - 1e-3 * WALK_SPEED_M_S / WALK_STRIDE_M).max(0.0) * WALK_STRIDE_M
            / WALK_SPEED_M_S
            + 1e-3;
        s.apply_move(dt_finish, 0.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Stand);
        assert!((s.position - pos).length() < 1e-6);
    }

    #[test]
    fn walk_after_settled_stand_starts_at_phase_zero() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.1, 1.0, 0.0, false);
        let remain = 0.5 - s.walk_phase;
        let dt_finish = remain * WALK_STRIDE_M / WALK_SPEED_M_S + 1e-3;
        s.apply_move(dt_finish, 0.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Stand);

        s.apply_move(1e-3, 1.0, 0.0, false);
        let expect = (WALK_SPEED_M_S * 1e-3 / WALK_STRIDE_M).rem_euclid(1.0);
        assert!(
            (s.walk_phase - expect).abs() < 1e-5,
            "phase={}",
            s.walk_phase
        );
    }

    #[test]
    fn wish_during_stopping_resumes_walk() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.15, 1.0, 0.0, false);
        s.apply_move(1e-3, 0.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Stopping);
        let phase = s.walk_phase;
        s.apply_move(1e-3, 1.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Walk);
        assert!(s.walk_phase > phase);
    }

    #[test]
    fn jump_launches_to_air_and_peaks_near_target() {
        let mut s = SelfState::default_loadout();
        s.try_jump();
        assert_eq!(s.locomotion, LocomotionMode::Air);
        assert!((s.velocity_y - JUMP_LAUNCH_M_S).abs() < 1e-5);

        let dt = 1.0 / 120.0;
        let mut peak = 0.0_f32;
        for _ in 0..200 {
            s.apply_move(dt, 0.0, 0.0, false);
            peak = peak.max(s.position.y);
            if s.is_grounded() {
                break;
            }
        }
        assert!(
            (peak - JUMP_PEAK_M).abs() < 0.05,
            "peak={peak} want ~{JUMP_PEAK_M}"
        );
        assert_eq!(s.locomotion, LocomotionMode::Stand);
        assert!(s.position.y.abs() < 1e-5);
        assert!(s.velocity_y.abs() < 1e-5);
    }

    #[test]
    fn jump_while_airborne_is_ignored() {
        let mut s = SelfState::default_loadout();
        s.try_jump();
        s.apply_move(0.05, 0.0, 0.0, false);
        let y = s.position.y;
        let vy = s.velocity_y;
        s.try_jump();
        assert!((s.position.y - y).abs() < 1e-6);
        assert!((s.velocity_y - vy).abs() < 1e-6);
    }

    #[test]
    fn air_coasts_at_launch_direction_and_freezes_phase() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.1, 1.0, 0.0, false);
        let phase = s.walk_phase;
        s.try_jump();
        // Strafe mid-air must not change path — still +Z from launch.
        s.apply_move(0.2, 0.0, 1.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Air);
        assert!((s.walk_phase - phase).abs() < 1e-6, "phase must freeze");
        assert!(
            (s.position.z - WALK_SPEED_M_S * 0.3).abs() < 1e-3,
            "z={}",
            s.position.z
        );
        assert!(s.position.x.abs() < 1e-3, "x={}", s.position.x);
    }

    #[test]
    fn land_with_wish_enters_walk() {
        let mut s = SelfState::default_loadout();
        s.try_jump();
        let dt = 1.0 / 60.0;
        for _ in 0..120 {
            s.apply_move(dt, 1.0, 0.0, false);
            if !s.locomotion.is_air() {
                break;
            }
        }
        assert_eq!(s.locomotion, LocomotionMode::Walk);
        assert!(s.position.y.abs() < 1e-5);
        assert!((s.walk_phase).abs() < 1e-6);
    }

    #[test]
    fn sprint_moves_faster_than_walk() {
        let mut walk = SelfState::default_loadout();
        let mut sprint = SelfState::default_loadout();
        walk.apply_move(1.0, 1.0, 0.0, false);
        sprint.apply_move(1.0, 1.0, 0.0, true);
        assert_eq!(sprint.locomotion, LocomotionMode::Sprint);
        assert!(sprint.sprint_latched);
        assert!((walk.position.z - WALK_SPEED_M_S).abs() < 1e-4);
        assert!((sprint.position.z - SPRINT_SPEED_M_S).abs() < 1e-4);
        assert!(sprint.stamina < STAMINA_MAX);
    }

    #[test]
    fn sprint_stays_without_holding() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.05, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
        s.apply_move(0.2, 1.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
        assert!(s.sprint_latched);
    }

    #[test]
    fn second_tap_keeps_sprint() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.05, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
        s.apply_move(0.05, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
        assert!(s.sprint_latched);
    }

    #[test]
    fn sprint_requires_min_stamina_to_start() {
        let mut s = SelfState::default_loadout();
        s.stamina = STAMINA_MIN_TO_START - 0.01;
        s.apply_move(0.1, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Walk);
        assert!(!s.sprint_latched);
        assert!((s.position.z - WALK_SPEED_M_S * 0.1).abs() < 1e-4);
    }

    #[test]
    fn sprint_continues_below_min_until_empty() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.05, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
        s.stamina = STAMINA_MIN_TO_START - 0.05;
        s.apply_move(0.05, 1.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
    }

    #[test]
    fn empty_stamina_drops_to_walk_without_restart() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.05, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
        s.stamina = 1e-4;
        s.apply_move(0.05, 1.0, 0.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Walk);
        assert!(!s.sprint_latched);
        assert!(s.stamina < STAMINA_MIN_TO_START);
        let z = s.position.z;
        // Fresh tap still blocked until min fill.
        s.apply_move(0.1, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Walk);
        assert!((s.position.z - z - WALK_SPEED_M_S * 0.1).abs() < 1e-3);
    }

    #[test]
    fn stamina_regens_when_not_sprinting() {
        let mut s = SelfState::default_loadout();
        s.stamina = 0.0;
        s.apply_move(STAMINA_REGEN_S, 0.0, 0.0, false);
        assert!((s.stamina - STAMINA_MAX).abs() < 1e-4);
    }

    #[test]
    fn stop_wish_clears_sprint_latch() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.1, 1.0, 0.0, true);
        assert!(s.sprint_latched);
        s.apply_move(0.1, 0.0, 0.0, false);
        assert!(!s.sprint_latched);
        assert_ne!(s.locomotion, LocomotionMode::Sprint);
    }

    #[test]
    fn air_does_not_start_sprint() {
        let mut s = SelfState::default_loadout();
        s.try_jump();
        s.apply_move(0.05, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Air);
    }

    #[test]
    fn jump_from_sprint_locks_sprint_air_speed() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.1, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
        s.try_jump();
        assert!((s.air_vel_z - SPRINT_SPEED_M_S).abs() < 1e-4);
        assert!(s.air_vel_x.abs() < 1e-5);
    }

    #[test]
    fn sprint_rejects_strafe_only_and_back() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.1, 0.0, 1.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Walk);
        assert!(!s.sprint_latched);
        assert!((s.position.length() - WALK_SPEED_M_S * 0.1).abs() < 1e-3);

        s = SelfState::default_loadout();
        s.apply_move(0.1, -1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Walk);
        assert!(!s.sprint_latched);
    }

    #[test]
    fn sprint_allows_forward_with_strafe() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.1, 1.0, 1.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
        assert!((s.position.length() - SPRINT_SPEED_M_S * 0.1).abs() < 1e-3);
    }

    #[test]
    fn losing_forward_ends_sprint() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.05, 1.0, 0.0, true);
        assert_eq!(s.locomotion, LocomotionMode::Sprint);
        s.apply_move(0.05, 0.0, 1.0, false);
        assert_eq!(s.locomotion, LocomotionMode::Walk);
        assert!(!s.sprint_latched);
    }

    #[test]
    fn latched_sprint_drains_in_air_no_jump_regen() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.05, 1.0, 0.0, true);
        assert!(s.sprint_latched);
        let before = s.stamina;
        s.try_jump();
        assert_eq!(s.locomotion, LocomotionMode::Air);
        // Full hop time is short; still must drain, never regen.
        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            s.apply_move(dt, 1.0, 0.0, false);
        }
        assert!(
            s.stamina < before - 0.05,
            "stamina={before} -> {} (expected drain while latched aloft)",
            s.stamina
        );
        assert!(s.stamina < before);
    }

    #[test]
    fn weapon_class_map_covers_a_through_r() {
        assert_eq!(WeaponClass::from_letter(b'a'), Some(WeaponClass::Launcher));
        assert_eq!(WeaponClass::from_letter(b'b'), Some(WeaponClass::Pistol));
        assert_eq!(WeaponClass::from_letter(b'p'), Some(WeaponClass::Smg));
        assert_eq!(WeaponClass::from_letter(b'z'), None);
        assert!(WeaponClass::Launcher.allowed_in_secondary());
        assert!(WeaponClass::Pistol.allowed_in_secondary());
        assert!(!WeaponClass::Smg.allowed_in_secondary());
    }

    #[test]
    fn secondary_rejects_non_sidearm() {
        let mut s = SelfState::default_loadout();
        assert!(s.set_secondary(Some(b'p')).is_err());
        assert_eq!(s.secondary, Some(b'b'));
        assert!(s.set_secondary(Some(b'i')).is_ok());
        assert_eq!(s.secondary, Some(b'i'));
        assert!(s.set_secondary(None).is_ok());
        assert_eq!(s.secondary, None);
    }

    #[test]
    fn primary_accepts_any_class() {
        let mut s = SelfState::default_loadout();
        assert!(s.set_primary(Some(b'a')).is_ok());
        assert_eq!(s.primary, Some(b'a'));
        assert!(s.set_primary(Some(b'e')).is_ok());
        assert!(s.set_primary(None).is_ok());
        assert_eq!(s.primary, None);
        // Still on primary slot — empty means unarmed, not a third mode.
        assert_eq!(s.active, ActiveWeapon::Primary);
        assert!(!s.is_armed());
    }

    #[test]
    fn cycle_weapon_toggles_two_slots_only() {
        let mut s = SelfState::default_loadout();
        assert_eq!(s.active, ActiveWeapon::Primary);
        s.cycle_weapon(1);
        assert_eq!(s.active, ActiveWeapon::Secondary);
        assert_eq!(s.active_blaster(), Some(b'b'));
        s.cycle_weapon(1);
        assert_eq!(s.active, ActiveWeapon::Primary);
        assert_eq!(s.active_blaster(), Some(b'p'));
        // Both filled → always armed; no free third unarmed step.
        assert!(s.is_armed());

        s.set_secondary(None).unwrap();
        s.active = ActiveWeapon::Primary;
        s.cycle_weapon(1);
        assert_eq!(s.active, ActiveWeapon::Secondary);
        assert!(!s.is_armed());
        assert!(s.reticle_world(Vec3::new(0.0, 1.5, 0.0)).is_none());
        s.cycle_weapon(-1);
        assert_eq!(s.active, ActiveWeapon::Primary);
        assert!(s.is_armed());
    }

    #[test]
    fn continuous_jump_while_latched_empties_stamina() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.02, 1.0, 0.0, true);
        let dt = 1.0 / 60.0;
        for _ in 0..500 {
            if s.is_grounded() {
                s.try_jump();
            }
            s.apply_move(dt, 1.0, 0.0, false);
            if !s.sprint_latched && s.stamina <= 0.0 {
                break;
            }
        }
        assert!(
            !s.sprint_latched && s.stamina <= 1e-5,
            "latch={} stamina={}",
            s.sprint_latched,
            s.stamina
        );
    }

    #[test]
    fn emote_commit_requires_ground_and_clears_sprint() {
        let mut s = SelfState::default_loadout();
        s.apply_move(0.05, 1.0, 0.0, true);
        assert!(s.sprint_latched);
        assert!(s.try_commit_emote(0, false));
        assert_eq!(s.emote, Some(0));
        assert!(!s.sprint_latched);
        assert!(s.emote_holster());
        assert!(!s.presents_armed());
        assert!(s.is_armed());
    }

    #[test]
    fn emote_blocked_in_air_and_by_weapon_side() {
        let mut s = SelfState::default_loadout();
        s.try_jump();
        assert!(!s.try_commit_emote(0, false));
        s = SelfState::default_loadout();
        assert!(!s.try_commit_emote(0, true));
    }

    #[test]
    fn emote_ends_after_duration_and_move_cancels() {
        let mut s = SelfState::default_loadout();
        assert!(s.try_commit_emote(3, false)); // bow 0.33s
        s.tick_emote(0.2);
        assert!(s.is_emoting());
        s.tick_emote(0.2);
        assert!(!s.is_emoting());

        assert!(s.try_commit_emote(0, false));
        s.apply_move(0.01, 1.0, 0.0, false);
        assert!(!s.is_emoting());
    }

    #[test]
    fn jump_and_cycle_cancel_emote() {
        let mut s = SelfState::default_loadout();
        assert!(s.try_commit_emote(1, false));
        s.try_jump();
        assert!(!s.is_emoting());

        s = SelfState::default_loadout();
        assert!(s.try_commit_emote(2, false));
        s.cycle_weapon(1);
        assert!(!s.is_emoting());
    }
}
