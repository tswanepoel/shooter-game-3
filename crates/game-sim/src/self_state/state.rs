//! SelfState fields, loadout, health, and emote.

use glam::Vec3;

use super::loco::STAMINA_MAX;
use super::pose::HIT_FALL_S;
use super::types::{ActiveWeapon, LocomotionMode, WeaponClass};

#[derive(Debug, Clone, PartialEq)]
pub struct SelfState {
    pub position: Vec3,
    /// Look azimuth (radians). 0 faces **+Z**.
    pub ocular_yaw: f32,
    /// Look elevation (radians). Positive looks up. Clamped to ┬▒90┬░.
    pub ocular_pitch: f32,
    pub character: u8,
    /// Primary slot blaster letter (`a`ΓÇª`r`), or empty (021).
    pub primary: Option<u8>,
    /// Secondary slot blaster letter; launcher/pistol only when set (021).
    pub secondary: Option<u8>,
    /// Which hand is active: a filled slot or unarmed (021).
    pub active: ActiveWeapon,
    pub alive: bool,
    pub health: f32,
    pub regen_block_s: f32,
    pub die_age_s: f32,

    /// Look-relative forward wish (ΓêÆ1ΓÇª1). Positive is W.
    pub wish_forward: f32,
    /// Look-relative strafe wish (ΓêÆ1ΓÇª1). Positive is D (right).
    pub wish_strafe: f32,
    pub locomotion: LocomotionMode,
    /// Fraction through the locomotion cycle, [0, 1).
    pub walk_phase: f32,
    /// Sprint stamina 0ΓÇª[`STAMINA_MAX`] (020).
    pub stamina: f32,
    /// Sticky sprint wish from Shift tap (020); cleared on cancel, stop, or empty bar.
    pub sprint_latched: bool,
    pub velocity_y: f32,
    /// Horizontal air velocity locked at jump (world XZ).
    pub air_vel_x: f32,
    pub air_vel_z: f32,

    /// Facing yaw (tracks look yaw for now).
    pub torso_yaw: f32,
    /// Hip fold (look-offset + fire/hit residual).
    pub torso_pitch: f32,
    /// Right-shoulder fold (look-offset + fire/hit/sway residual).
    pub shoulder_pitch: f32,
    /// Right-shoulder twist (fire/hit/sway residual).
    pub shoulder_yaw: f32,
    /// Neck twist relative to torso.
    pub head_yaw: f32,
    /// Neck fold (look-offset + fire/hit residual).
    pub head_pitch: f32,

    pub hip_fire_fold: f32,
    pub hip_hit_fold: f32,
    pub shoulder_fire_fold: f32,
    pub shoulder_fire_twist: f32,
    pub shoulder_hit_fold: f32,
    pub shoulder_hit_twist: f32,
    pub shoulder_sway_fold: f32,
    pub shoulder_sway_twist: f32,
    pub neck_fire_fold: f32,
    pub neck_hit_fold: f32,
    /// Grip socket bore travel from fire impulse (m along bore, + back).
    pub grip_bore_m: f32,

    /// Base fire residual fall time (s), from last fire impulse size.
    pub fire_fall_s: f32,
    pub hit_fall_s: f32,

    /// Active emote wheel slot (`0`ΓÇª`3`), if any (039).
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
            health: crate::HEALTH_MAX,
            regen_block_s: 0.0,
            die_age_s: 0.0,
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
            shoulder_yaw: 0.0,
            head_yaw: 0.0,
            head_pitch: 0.0,
            hip_fire_fold: 0.0,
            hip_hit_fold: 0.0,
            shoulder_fire_fold: 0.0,
            shoulder_fire_twist: 0.0,
            shoulder_hit_fold: 0.0,
            shoulder_hit_twist: 0.0,
            shoulder_sway_fold: 0.0,
            shoulder_sway_twist: 0.0,
            neck_fire_fold: 0.0,
            neck_hit_fold: 0.0,
            grip_bore_m: 0.0,
            fire_fall_s: 0.08,
            hit_fall_s: HIT_FALL_S,
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

    /// Present-armed: living, active letter filled, and not holstered for emote.
    pub fn presents_armed(&self) -> bool {
        self.alive && self.is_armed() && !self.emote_holster()
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

    pub fn apply_damage(&mut self, amount: f32) {
        if !self.alive || amount <= 0.0 {
            return;
        }
        self.health = (self.health - amount).max(0.0);
        self.regen_block_s = crate::HEALTH_REGEN_DELAY_S;
        if self.health <= 0.0 {
            self.health = 0.0;
            self.alive = false;
            self.die_age_s = 0.0;
            self.sprint_latched = false;
            self.clear_emote();
            self.wish_forward = 0.0;
            self.wish_strafe = 0.0;
            self.clear_joint_residual();
            if !self.locomotion.is_air() {
                self.locomotion = LocomotionMode::Stand;
                self.walk_phase = 0.0;
            }
        }
    }

    pub fn tick_health(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        if !self.alive {
            self.die_age_s += dt;
            return;
        }
        if self.regen_block_s > 0.0 {
            self.regen_block_s = (self.regen_block_s - dt).max(0.0);
            return;
        }
        if self.health < crate::HEALTH_MAX && crate::HEALTH_REGEN_FULL_S > 1e-6 {
            let rate = crate::HEALTH_MAX / crate::HEALTH_REGEN_FULL_S;
            self.health = (self.health + rate * dt).min(crate::HEALTH_MAX);
        }
    }

    /// Commit a wheel slot. Requires grounded; `weapon_side_blocked` is burst (038).
    /// Replaces an in-flight emote. Clears sprint latch.
    pub fn try_commit_emote(&mut self, id: u8, weapon_side_blocked: bool) -> bool {
        if !self.alive || weapon_side_blocked || !self.is_grounded() {
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

    /// Toggle active slot: primary Γåö secondary. Empty slots stay in the cycle (unarmed).
    /// Cancels emote (039) so the new hand is free immediately.
    pub fn cycle_weapon(&mut self, dir: i8) {
        if !self.alive || dir.signum() == 0 {
            return;
        }
        self.clear_emote();
        self.active = match self.active {
            ActiveWeapon::Primary => ActiveWeapon::Secondary,
            ActiveWeapon::Secondary => ActiveWeapon::Primary,
        };
    }
}
