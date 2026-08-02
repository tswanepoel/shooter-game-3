//! SelfState fields, loadout, health, ammo, and emote.

use glam::Vec3;

use super::loco::STAMINA_MAX;
use super::pose::HIT_FALL_S;
use super::types::{prefer_armed_slot, ActiveWeapon, LocomotionMode, WeaponClass};
use crate::{weapon_def, AmmoKind, ReserveAmmo};

#[derive(Debug, Clone, PartialEq)]
pub struct SelfState {
    pub position: Vec3,
    /// Placement root yaw. 0 faces **+Z**.
    pub facing: f32,
    /// Look azimuth relative to facing (radians). Currently forced 0 (facing tracks look).
    pub look_offset_yaw: f32,
    /// Look elevation relative to facing (radians). Positive up; clamped ±90°.
    pub look_offset_pitch: f32,
    pub character: u8,
    /// Primary slot blaster letter (`a`…`r`), or empty (021).
    pub primary: Option<u8>,
    /// Secondary slot blaster letter; launcher/pistol only when set (021).
    pub secondary: Option<u8>,
    /// Which hand is active: a filled slot or unarmed (021).
    pub active: ActiveWeapon,
    pub primary_mag: u16,
    pub secondary_mag: u16,
    pub primary_chamber: u16,
    pub secondary_chamber: u16,
    pub reserve: ReserveAmmo,
    pub alive: bool,
    pub health: f32,
    pub regen_block_s: f32,
    pub die_age_s: f32,

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
    /// Remaining coyote grace after leaving support (seconds).
    pub coyote_s: f32,
    /// Remaining buffered jump press while airborne (seconds).
    pub jump_buffer_s: f32,

    /// Hip fold (look-offset share + residual). Present maps onto kit torso.
    pub hip_fold: f32,
    /// Right-shoulder fold (look-offset share + residual + sway).
    pub shoulder_fold: f32,
    /// Right-shoulder twist (residual + sway).
    pub shoulder_twist: f32,
    /// Neck twist relative to torso (look-offset yaw share).
    pub neck_twist: f32,
    /// Neck fold (look-offset share + residual). Present maps onto kit head.
    pub neck_fold: f32,

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
        let mut s = Self {
            position: Vec3::ZERO,
            facing: 0.0,
            look_offset_yaw: 0.0,
            look_offset_pitch: 0.0,
            character: b'a',
            primary: Some(b'p'),
            secondary: Some(b'b'),
            active: ActiveWeapon::Primary,
            primary_mag: 0,
            secondary_mag: 0,
            primary_chamber: 0,
            secondary_chamber: 0,
            reserve: ReserveAmmo::default(),
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
            coyote_s: 0.0,
            jump_buffer_s: 0.0,
            hip_fold: 0.0,
            shoulder_fold: 0.0,
            shoulder_twist: 0.0,
            neck_twist: 0.0,
            neck_fold: 0.0,
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
        };
        s.apply_spawn_ammo();
        s
    }

    /// Per ammo kind: max spare among loadout letters that use that kind. Chamber starts empty.
    pub fn apply_spawn_ammo(&mut self) {
        self.primary_mag = Self::spawn_mag_of(self.primary);
        self.secondary_mag = Self::spawn_mag_of(self.secondary);
        self.primary_chamber = 0;
        self.secondary_chamber = 0;
        self.reserve = ReserveAmmo::default();
        for letter in [self.primary, self.secondary].into_iter().flatten() {
            let Some(def) = weapon_def(letter) else {
                continue;
            };
            let spare = crate::spawn_spare_for_letter(letter);
            let kind = def.ammo();
            let have = self.reserve.get(kind);
            if spare > have {
                self.reserve.set(kind, spare);
            }
        }
    }

    fn mag_capacity_of(letter: Option<u8>) -> u16 {
        letter
            .and_then(weapon_def)
            .map(|d| d.mag_capacity())
            .unwrap_or(0)
    }

    fn spawn_mag_of(letter: Option<u8>) -> u16 {
        letter.map(crate::spawn_mag_for_letter).unwrap_or(0)
    }

    fn chamber_capacity_of(letter: Option<u8>) -> u16 {
        letter
            .and_then(weapon_def)
            .map(|d| d.chamber_capacity())
            .unwrap_or(0)
    }

    pub fn active_mag(&self) -> Option<u16> {
        self.active_blaster()?;
        Some(match self.active {
            ActiveWeapon::Primary => self.primary_mag,
            ActiveWeapon::Secondary => self.secondary_mag,
        })
    }

    pub fn active_chamber(&self) -> Option<u16> {
        self.active_blaster()?;
        Some(match self.active {
            ActiveWeapon::Primary => self.primary_chamber,
            ActiveWeapon::Secondary => self.secondary_chamber,
        })
    }

    fn active_chamber_mut(&mut self) -> Option<&mut u16> {
        self.active_blaster()?;
        Some(match self.active {
            ActiveWeapon::Primary => &mut self.primary_chamber,
            ActiveWeapon::Secondary => &mut self.secondary_chamber,
        })
    }

    pub fn active_mag_capacity(&self) -> Option<u16> {
        self.active_blaster()
            .and_then(weapon_def)
            .map(|d| d.mag_capacity())
    }

    pub fn active_ammo_kind(&self) -> Option<AmmoKind> {
        self.active_blaster().and_then(weapon_def).map(|d| d.ammo())
    }

    pub fn spend_chamber_rounds(&mut self, want: u16) -> u16 {
        let Some(ch) = self.active_chamber_mut() else {
            return 0;
        };
        let spent = (*ch).min(want);
        *ch -= spent;
        spent
    }

    pub fn feed_chamber_from_mag(&mut self, want: u16) -> u16 {
        let letter = self.active_blaster();
        let cap = Self::chamber_capacity_of(letter);
        let mag_cap = Self::mag_capacity_of(letter);
        if cap == 0 || want == 0 || mag_cap == 0 {
            return 0;
        }
        let mag_n = match self.active {
            ActiveWeapon::Primary => self.primary_mag,
            ActiveWeapon::Secondary => self.secondary_mag,
        };
        let ch = match self.active {
            ActiveWeapon::Primary => self.primary_chamber,
            ActiveWeapon::Secondary => self.secondary_chamber,
        };
        let n = want.min(mag_n).min(cap.saturating_sub(ch));
        if n == 0 {
            return 0;
        }
        match self.active {
            ActiveWeapon::Primary => {
                self.primary_mag -= n;
                self.primary_chamber += n;
            }
            ActiveWeapon::Secondary => {
                self.secondary_mag -= n;
                self.secondary_chamber += n;
            }
        }
        n
    }

    pub fn feed_chamber_from_reserve(&mut self, want: u16) -> u16 {
        let Some(letter) = self.active_blaster() else {
            return 0;
        };
        let Some(def) = weapon_def(letter) else {
            return 0;
        };
        if def.has_magazine() || want == 0 {
            return 0;
        }
        let cap = def.chamber_capacity();
        let ch = match self.active {
            ActiveWeapon::Primary => self.primary_chamber,
            ActiveWeapon::Secondary => self.secondary_chamber,
        };
        let room = cap.saturating_sub(ch);
        let n = want.min(room).min(self.reserve.get(def.ammo()));
        if n == 0 {
            return 0;
        }
        self.reserve.take(def.ammo(), n);
        match self.active {
            ActiveWeapon::Primary => self.primary_chamber += n,
            ActiveWeapon::Secondary => self.secondary_chamber += n,
        }
        n
    }

    /// Mag: room + mag rounds. No-mag: empty chamber + reserve (081).
    pub fn can_seat_chamber(&self) -> bool {
        let Some(letter) = self.active_blaster() else {
            return false;
        };
        let Some(def) = weapon_def(letter) else {
            return false;
        };
        let ch = match self.active {
            ActiveWeapon::Primary => self.primary_chamber,
            ActiveWeapon::Secondary => self.secondary_chamber,
        };
        if ch >= def.chamber_capacity() {
            return false;
        }
        if def.has_magazine() {
            let mag = match self.active {
                ActiveWeapon::Primary => self.primary_mag,
                ActiveWeapon::Secondary => self.secondary_mag,
            };
            mag > 0
        } else {
            ch == 0 && self.reserve.get(def.ammo()) > 0
        }
    }

    /// Room left in reserve for `kind` under capacity (059).
    pub fn reserve_room(&self, kind: AmmoKind) -> u16 {
        self.reserve.room(kind)
    }

    /// Grant rounds into reserve, capped by capacity. Returns how many added (059).
    pub fn grant_reserve(&mut self, kind: AmmoKind, n: u16) -> u16 {
        self.reserve.add_capped(kind, n)
    }

    /// Empty active kind's reserve into a death ammo dump (059 / 067). Magazine stays
    /// with the blaster drop. Returns `None` when there is no active blaster.
    /// Rounds may be zero (no ammo drop).
    pub fn dump_death_ammo(&mut self) -> Option<(AmmoKind, u16)> {
        let kind = self.active_ammo_kind()?;
        let from_reserve = self.reserve.take(kind, u16::MAX);
        Some((kind, from_reserve))
    }

    /// Strip the active blaster letter + magazine + chamber for a floor drop (067).
    /// Mag and chamber are separate real state — no fold. Clears that slot.
    /// Returns `None` when unarmed.
    pub fn take_active_blaster_drop(&mut self) -> Option<(u8, u16, u16)> {
        let letter = self.active_blaster()?;
        let (mag, chamber) = match self.active {
            ActiveWeapon::Primary => {
                let m = self.primary_mag;
                let c = self.primary_chamber;
                self.primary = None;
                self.primary_mag = 0;
                self.primary_chamber = 0;
                (m, c)
            }
            ActiveWeapon::Secondary => {
                let m = self.secondary_mag;
                let c = self.secondary_chamber;
                self.secondary = None;
                self.secondary_mag = 0;
                self.secondary_chamber = 0;
                (m, c)
            }
        };
        Some((letter, mag, chamber))
    }

    /// Equip a floor blaster (letter + mag + chamber) into a slot (067).
    /// Prefer free primary, then free secondary; else swap active.
    /// Secondary may hold any class on floor grant (021 laws stay on loadout/spawn).
    /// Mag and chamber carry over separately (no fold/unfold); each clamps to that
    /// letter's own capacity. Returns displaced `(letter, mag, chamber)` on a swap.
    pub fn grant_floor_blaster(
        &mut self,
        letter: u8,
        mag: u16,
        chamber: u16,
    ) -> Result<Option<(u8, u16, u16)>, &'static str> {
        if !self.alive {
            return Err("dead");
        }
        WeaponClass::from_letter(letter).ok_or("unknown blaster letter")?;
        self.clear_emote();

        if self.primary.is_none() {
            self.write_slot(ActiveWeapon::Primary, Some(letter), mag, chamber);
            self.active = ActiveWeapon::Primary;
            return Ok(None);
        }
        if self.secondary.is_none() {
            self.write_slot(ActiveWeapon::Secondary, Some(letter), mag, chamber);
            self.active = ActiveWeapon::Secondary;
            return Ok(None);
        }

        let slot = self.active;
        let displaced = self.read_slot(slot);
        self.write_slot(slot, Some(letter), mag, chamber);
        self.active = slot;
        Ok(displaced)
    }

    fn read_slot(&self, slot: ActiveWeapon) -> Option<(u8, u16, u16)> {
        match slot {
            ActiveWeapon::Primary => self
                .primary
                .map(|l| (l, self.primary_mag, self.primary_chamber)),
            ActiveWeapon::Secondary => self
                .secondary
                .map(|l| (l, self.secondary_mag, self.secondary_chamber)),
        }
    }

    /// Clamp `mag` / `chamber` to `letter`'s own capacities independently. No
    /// mag-fed-forces-chamber-zero or no-mag-stuffs-mag-into-chamber unfold.
    fn write_slot(&mut self, slot: ActiveWeapon, letter: Option<u8>, mag: u16, chamber: u16) {
        let (mag, chamber) = if letter.is_none() {
            (0, 0)
        } else {
            (
                mag.min(Self::mag_capacity_of(letter)),
                chamber.min(Self::chamber_capacity_of(letter)),
            )
        };
        match slot {
            ActiveWeapon::Primary => {
                self.primary = letter;
                self.primary_mag = mag;
                self.primary_chamber = chamber;
            }
            ActiveWeapon::Secondary => {
                self.secondary = letter;
                self.secondary_mag = mag;
                self.secondary_chamber = chamber;
            }
        }
    }

    /// No-mag never reloads (081).
    pub fn can_reload(&self) -> bool {
        if !self.alive {
            return false;
        }
        let Some(def) = self.active_blaster().and_then(weapon_def) else {
            return false;
        };
        if !def.has_magazine() {
            return false;
        }
        let mag = match self.active {
            ActiveWeapon::Primary => self.primary_mag,
            ActiveWeapon::Secondary => self.secondary_mag,
        };
        mag < def.mag_capacity() && self.reserve.get(def.ammo()) > 0
    }

    pub fn try_reload(&mut self) -> bool {
        if !self.alive {
            return false;
        }
        let Some(letter) = self.active_blaster() else {
            return false;
        };
        let Some(def) = weapon_def(letter) else {
            return false;
        };
        if !def.has_magazine() {
            return false;
        }
        let cap = def.mag_capacity();
        let kind = def.ammo();
        let mag = match self.active {
            ActiveWeapon::Primary => &mut self.primary_mag,
            ActiveWeapon::Secondary => &mut self.secondary_mag,
        };
        if *mag >= cap {
            return false;
        }
        let need = cap - *mag;
        let taken = self.reserve.take(kind, need);
        if taken == 0 {
            return false;
        }
        *mag += taken;
        true
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
    /// Fills that slot's magazine to capacity when the letter changes (058).
    pub fn set_primary(&mut self, letter: Option<u8>) -> Result<(), &'static str> {
        if let Some(l) = letter {
            WeaponClass::from_letter(l).ok_or("unknown blaster letter")?;
        }
        if self.primary != letter {
            self.primary = letter;
            self.primary_mag = Self::mag_capacity_of(letter);
            self.primary_chamber = 0;
        }
        Ok(())
    }

    /// Set secondary (launcher/pistol only, or clear). Invalid class rejected.
    /// Fills that slot's magazine to capacity when the letter changes (058).
    pub fn set_secondary(&mut self, letter: Option<u8>) -> Result<(), &'static str> {
        if let Some(l) = letter {
            let class = WeaponClass::from_letter(l).ok_or("unknown blaster letter")?;
            if !class.allowed_in_secondary() {
                return Err("secondary only allows launcher or pistol");
            }
        }
        if self.secondary != letter {
            self.secondary = letter;
            self.secondary_mag = Self::mag_capacity_of(letter);
            self.secondary_chamber = 0;
        }
        Ok(())
    }

    /// Prefer a filled slot when the active hand is empty but the other holds a blaster (081).
    pub fn coerce_active_armed(&mut self) {
        self.active = prefer_armed_slot(self.primary, self.secondary, self.active);
    }

    /// Toggle active slot: primary ↔ secondary.
    /// Skips an empty slot while the other hand still holds a blaster (081).
    /// Cancels emote (039) so the new hand is free immediately.
    pub fn cycle_weapon(&mut self, dir: i8) {
        if !self.alive || dir.signum() == 0 {
            return;
        }
        self.clear_emote();
        let next = match self.active {
            ActiveWeapon::Primary => ActiveWeapon::Secondary,
            ActiveWeapon::Secondary => ActiveWeapon::Primary,
        };
        let next_filled = match next {
            ActiveWeapon::Primary => self.primary.is_some(),
            ActiveWeapon::Secondary => self.secondary.is_some(),
        };
        if !next_filled && self.is_armed() {
            return;
        }
        self.active = next;
    }
}
