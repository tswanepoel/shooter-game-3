//! Weapon fire gates, modes, and projectile motion (038).

use glam::{Quat, Vec3};

use crate::weapons::{
    weapon_def, FireMode, MuzzlePolicy, WeaponDef, PROJECTILE_GRAVITY, SPRINT_FIRE_BASE_S,
};
use crate::{ActiveWeapon, SelfState, WeaponClass};

/// One projectile in flight (anemic bag; motion rules live on [`ProjectileWorld`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Projectile {
    pub id: u64,
    /// Shooter id (0 in solo when not networked).
    pub owner: u32,
    pub weapon: u8,
    pub origin: Vec3,
    pub position: Vec3,
    pub velocity: Vec3,
    /// Path length from origin (m).
    pub traveled: f32,
    pub max_range: f32,
    /// Kit muzzle index that spawned this (present flash).
    pub muzzle_index: u8,
}

/// World set of live projectiles (self-claimed + accepted peer spawns).
#[derive(Debug, Clone, Default)]
pub struct ProjectileWorld {
    pub projectiles: Vec<Projectile>,
}

impl ProjectileWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self, p: Projectile) {
        self.projectiles.push(p);
    }

    pub fn spawn_many(&mut self, iter: impl IntoIterator<Item = Projectile>) {
        self.projectiles.extend(iter);
    }

    /// Gravity step + despawn when path length reaches max range.
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        for p in &mut self.projectiles {
            p.velocity += PROJECTILE_GRAVITY * dt;
            let step = p.velocity * dt;
            p.position += step;
            p.traveled += step.length();
        }
        self.projectiles.retain(|p| p.traveled < p.max_range);
    }

    pub fn clear(&mut self) {
        self.projectiles.clear();
    }
}

/// One accepted discharge: projectiles + present cues.
#[derive(Debug, Clone)]
pub struct Discharge {
    pub weapon: u8,
    pub projectiles: Vec<Projectile>,
    /// Muzzle indices that fired (unique, for flash).
    pub fired_muzzles: Vec<u8>,
}

/// Fire cadence / gates for one self (038).
#[derive(Debug, Clone)]
pub struct FireState {
    /// Remaining weapon-ready gate (s).
    ready_s: f32,
    /// Remaining sprint→fire tax (s). Includes base + letter ready when armed.
    sprint_fire_s: f32,
    /// Time until next discharge allowed by RPM (s).
    cooldown_s: f32,
    /// Shots left in the current AR string (0 = idle).
    burst_left: u8,
    /// Mid-string re-press: one follow-up string when current ends.
    burst_pending: bool,
    /// Hold-to-chain: LMB still down at string end.
    fire_held: bool,
    /// Previous frame held (edge detect when edge not supplied separately).
    prev_held: bool,
    /// Round-robin muzzle cursor.
    alt_muzzle: u8,
    /// Letter that last paid ready / last fired.
    armed_letter: Option<u8>,
    next_id: u64,
    /// Simple LCG for spread (deterministic).
    rng: u32,
}

impl Default for FireState {
    fn default() -> Self {
        Self::new()
    }
}

impl FireState {
    pub fn new() -> Self {
        Self {
            ready_s: 0.0,
            sprint_fire_s: 0.0,
            cooldown_s: 0.0,
            burst_left: 0,
            burst_pending: false,
            fire_held: false,
            prev_held: false,
            alt_muzzle: 0,
            armed_letter: None,
            next_id: 1,
            rng: 0xC0FFEE42,
        }
    }

    pub fn burst_active(&self) -> bool {
        self.burst_left > 0
    }

    /// Weapon-side actions (sprint, wheel, equip) wait while a string runs.
    pub fn blocks_weapon_side(&self) -> bool {
        self.burst_active()
    }

    /// Pay letter ready after equip / swap / spawn onto a letter.
    pub fn pay_ready(&mut self, letter: u8) {
        if let Some(def) = weapon_def(letter) {
            self.ready_s = def.t_ready;
            self.armed_letter = Some(letter);
            self.burst_left = 0;
            self.burst_pending = false;
            self.cooldown_s = 0.0;
            self.alt_muzzle = 0;
        }
    }

    /// Sync with loadout: pay ready when active letter changes (incl. unarmed).
    pub fn sync_active_letter(&mut self, letter: Option<u8>) {
        if letter != self.armed_letter {
            match letter {
                Some(l) => self.pay_ready(l),
                None => {
                    self.armed_letter = None;
                    self.ready_s = 0.0;
                    self.burst_left = 0;
                    self.burst_pending = false;
                }
            }
        }
    }

    /// Fire cleared sprint: start sprint→fire gate (base + letter ready).
    pub fn on_sprint_cleared_by_fire(&mut self, letter: u8) {
        let t_ready = weapon_def(letter).map(|d| d.t_ready).unwrap_or(0.0);
        self.sprint_fire_s = SPRINT_FIRE_BASE_S + t_ready;
    }

    fn gates_clear(&self) -> bool {
        self.ready_s <= 0.0 && self.sprint_fire_s <= 0.0 && self.cooldown_s <= 0.0
    }

    fn next_rand01(&mut self) -> f32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32) / (u32::MAX as f32)
    }

    /// Advance timers; consume fire input; maybe produce discharges.
    ///
    /// `muzzle_worlds`: kit muzzle points in world order (037 present pose).
    /// Empty → no spawn this frame (caller should supply when mesh ready).
    /// `owner`: projectile owner id for net claim (0 solo).
    pub fn tick(
        &mut self,
        dt: f32,
        self_state: &mut SelfState,
        fire_held: bool,
        owner: u32,
        aim: Vec3,
        muzzle_worlds: &[Vec3],
    ) -> Vec<Discharge> {
        let dt = dt.max(0.0);
        self.ready_s = (self.ready_s - dt).max(0.0);
        self.sprint_fire_s = (self.sprint_fire_s - dt).max(0.0);
        self.cooldown_s = (self.cooldown_s - dt).max(0.0);

        let letter = self_state.active_blaster();
        self.sync_active_letter(letter);

        let Some(letter) = letter else {
            self.fire_held = false;
            self.prev_held = false;
            self.burst_left = 0;
            self.burst_pending = false;
            return Vec::new();
        };
        let Some(def) = weapon_def(letter) else {
            return Vec::new();
        };

        let press_edge = fire_held && !self.prev_held;
        self.fire_held = fire_held;
        self.prev_held = fire_held;

        // Fire cancels emote (holster restore before muzzle spawn) and sprint (038/039).
        let fire_intent =
            press_edge || (fire_held && def.mode == FireMode::FullAuto) || self.burst_active();
        if fire_intent && self_state.is_emoting() {
            self_state.clear_emote();
        }
        if fire_intent && self_state.sprint_latched {
            self_state.sprint_latched = false;
            if self_state.locomotion.is_sprint() {
                self_state.locomotion = crate::LocomotionMode::Walk;
            }
            self.on_sprint_cleared_by_fire(letter);
        }

        // Mid-string re-press arms one follow-up.
        if self.burst_active() && press_edge {
            self.burst_pending = true;
        }

        let mut out = Vec::new();

        // Burst continuation (string always finishes).
        if self.burst_active() {
            if self.gates_clear() && !muzzle_worlds.is_empty() {
                if let Some(d) = self.spawn_discharge(def, owner, aim, muzzle_worlds) {
                    self.burst_left = self.burst_left.saturating_sub(1);
                    self.cooldown_s = def.shot_interval_s();
                    out.push(d);
                }
            }
            if self.burst_left == 0 {
                // String ended: hold-to-chain or pending re-press (next shots wait on RPM).
                let chain = self.fire_held || self.burst_pending;
                self.burst_pending = false;
                if chain {
                    self.burst_left = def.burst_count;
                }
            }
            return out;
        }

        // Start a new discharge / string.
        let want = match def.mode {
            FireMode::Semi => press_edge,
            FireMode::FullAuto => fire_held,
            FireMode::Burst => press_edge,
        };

        if want && self.gates_clear() && !muzzle_worlds.is_empty() {
            match def.mode {
                FireMode::Burst => {
                    self.burst_left = def.burst_count;
                    if let Some(d) = self.spawn_discharge(def, owner, aim, muzzle_worlds) {
                        self.burst_left = self.burst_left.saturating_sub(1);
                        self.cooldown_s = def.shot_interval_s();
                        out.push(d);
                    }
                }
                FireMode::Semi | FireMode::FullAuto => {
                    if let Some(d) = self.spawn_discharge(def, owner, aim, muzzle_worlds) {
                        self.cooldown_s = def.shot_interval_s();
                        out.push(d);
                    }
                }
            }
        }

        out
    }

    fn spawn_discharge(
        &mut self,
        def: &WeaponDef,
        owner: u32,
        aim: Vec3,
        muzzle_worlds: &[Vec3],
    ) -> Option<Discharge> {
        if muzzle_worlds.is_empty() {
            return None;
        }
        let aim = {
            let len = aim.length();
            if len < 1e-8 {
                Vec3::Z
            } else {
                aim / len
            }
        };

        let muzzle_indices: Vec<usize> = match def.muzzle_policy {
            MuzzlePolicy::Single => vec![0],
            MuzzlePolicy::All => (0..muzzle_worlds.len()).collect(),
            MuzzlePolicy::Alternate => {
                let n = muzzle_worlds.len().max(1);
                let i = (self.alt_muzzle as usize) % n;
                self.alt_muzzle = self.alt_muzzle.wrapping_add(1);
                vec![i]
            }
        };

        let mut projectiles = Vec::new();
        let mut fired_muzzles = Vec::new();

        for &mi in &muzzle_indices {
            let origin = muzzle_worlds[mi.min(muzzle_worlds.len() - 1)];
            fired_muzzles.push(mi as u8);
            for _ in 0..def.pellets {
                let dir = scatter_direction(aim, def.spread_half_deg, &mut || self.next_rand01());
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                projectiles.push(Projectile {
                    id,
                    owner,
                    weapon: def.letter,
                    origin,
                    position: origin,
                    velocity: dir * def.muzzle_vel,
                    traveled: 0.0,
                    max_range: def.max_range,
                    muzzle_index: mi as u8,
                });
            }
        }

        Some(Discharge {
            weapon: def.letter,
            projectiles,
            fired_muzzles,
        })
    }
}

/// Sample a unit direction inside a cone about `aim` (half-angle degrees).
fn scatter_direction(aim: Vec3, half_deg: f32, rand01: &mut dyn FnMut() -> f32) -> Vec3 {
    if half_deg <= 1e-6 {
        return aim;
    }
    let half = half_deg.to_radians();
    // Uniform in cone: cos(theta) between cos(half) and 1.
    let u = rand01();
    let v = rand01();
    let cos_max = half.cos();
    let cos_t = 1.0 - u * (1.0 - cos_max);
    let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
    let phi = v * std::f32::consts::TAU;

    // Orthonormal basis with aim as +Z of local cone.
    let up = if aim.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let right = aim.cross(up).normalize_or_zero();
    let bitangent = right.cross(aim).normalize_or_zero();
    let dir = (aim * cos_t + right * (sin_t * phi.cos()) + bitangent * (sin_t * phi.sin()))
        .normalize_or_zero();
    if dir.length_squared() < 1e-12 {
        aim
    } else {
        dir
    }
}

/// Equip a letter into the active slot, flipping primary↔secondary when 021 requires it.
///
/// Returns `Ok(true)` if loadout changed. Pays ready via [`FireState::sync_active_letter`]
/// on the next tick if letter changes.
pub fn equip_blaster_letter(state: &mut SelfState, letter: u8) -> Result<bool, &'static str> {
    let class = WeaponClass::from_letter(letter).ok_or("unknown blaster letter")?;
    state.clear_emote();
    let before = (
        state.primary,
        state.secondary,
        state.active,
        state.active_blaster(),
    );

    // Prefer keeping current active slot if class fits.
    let fits_secondary = class.allowed_in_secondary();
    match state.active {
        ActiveWeapon::Primary => {
            state.set_primary(Some(letter))?;
        }
        ActiveWeapon::Secondary => {
            if fits_secondary {
                state.set_secondary(Some(letter))?;
            } else {
                // Flip to primary and equip there.
                state.active = ActiveWeapon::Primary;
                state.set_primary(Some(letter))?;
            }
        }
    }

    let after = (
        state.primary,
        state.secondary,
        state.active,
        state.active_blaster(),
    );
    Ok(before != after)
}

/// Build a unit aim direction from look (015).
pub fn aim_from_self(state: &SelfState) -> Vec3 {
    state.ocular_forward()
}

/// Present-pose weapon jolt (038): mild kick on the held blaster, camera stays put.
#[derive(Debug, Clone, Copy, Default)]
pub struct JoltPose {
    pub pitch_rad: f32,
    pub yaw_rad: f32,
    pub back_m: f32,
}

impl JoltPose {
    /// Add one discharge kick. `yaw_sign` should be ±1 for left/right scatter.
    pub fn add_kick(&mut self, def: &WeaponDef, yaw_sign: f32) {
        let j = def.jolt;
        let sign = if yaw_sign >= 0.0 { 1.0 } else { -1.0 };
        self.pitch_rad += j.pitch_deg.to_radians();
        self.yaw_rad += j.yaw_deg.to_radians() * sign;
        self.back_m += j.back_m;
        // Clamp ~2.5× single kick so full-auto does not explode.
        self.pitch_rad = self.pitch_rad.min(j.pitch_deg.to_radians() * 2.5);
        self.yaw_rad = self
            .yaw_rad
            .clamp(-j.yaw_deg.to_radians() * 2.5, j.yaw_deg.to_radians() * 2.5);
        self.back_m = self.back_m.min(j.back_m * 2.5);
    }

    /// Recover toward rest over `settle_s` (exponential-ish linear blend).
    pub fn settle(&mut self, dt: f32, settle_s: f32) {
        if settle_s <= 1e-6 {
            *self = Self::default();
            return;
        }
        let k = (dt / settle_s).clamp(0.0, 1.0);
        self.pitch_rad *= 1.0 - k;
        self.yaw_rad *= 1.0 - k;
        self.back_m *= 1.0 - k;
        if self.pitch_rad.abs() < 1e-5 {
            self.pitch_rad = 0.0;
        }
        if self.yaw_rad.abs() < 1e-5 {
            self.yaw_rad = 0.0;
        }
        if self.back_m.abs() < 1e-6 {
            self.back_m = 0.0;
        }
    }

    /// Blaster-local kick about the **mesh origin** (pitch +X, yaw +Y, back +Z).
    /// Prefer [`Self::matrix_about_grip`] for present (038 / 037 hand socket).
    pub fn matrix(self) -> glam::Mat4 {
        let rot = Quat::from_rotation_y(self.yaw_rad) * Quat::from_rotation_x(self.pitch_rad);
        glam::Mat4::from_rotation_translation(rot, Vec3::new(0.0, 0.0, self.back_m))
    }

    /// Blaster-local kick pivoting on grip **G** (hand): `T(g) · J · T(−g)`.
    ///
    /// Compose as `held_blaster · matrix_about_grip(g)` so the fist stays put and the
    /// barrel kicks; mesh bore is −Z so back shove is +Z in blaster local.
    pub fn matrix_about_grip(self, grip_local: Vec3) -> glam::Mat4 {
        let t = glam::Mat4::from_translation(grip_local);
        t * self.matrix() * t.inverse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SelfState;

    fn armed_self() -> SelfState {
        SelfState::default_loadout()
    }

    fn muzzles() -> Vec<Vec3> {
        vec![Vec3::new(0.0, 1.4, 0.4)]
    }

    #[test]
    fn semi_fires_once_per_edge() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;

        let aim = Vec3::Z;
        let m = muzzles();
        // press
        let d0 = fire.tick(1.0 / 60.0, &mut s, true, 0, aim, &m);
        assert_eq!(d0.len(), 1);
        assert_eq!(d0[0].projectiles.len(), 1);
        // hold
        let d1 = fire.tick(1.0 / 60.0, &mut s, true, 0, aim, &m);
        assert!(d1.is_empty());
        // release + press after cooldown
        let _ = fire.tick(1.0, &mut s, false, 0, aim, &m);
        fire.cooldown_s = 0.0;
        let d2 = fire.tick(1.0 / 60.0, &mut s, true, 0, aim, &m);
        assert_eq!(d2.len(), 1);
    }

    #[test]
    fn full_auto_while_held() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        // p is SMG alternate
        fire.pay_ready(b'p');
        fire.ready_s = 0.0;
        let aim = Vec3::Z;
        let m = vec![Vec3::ZERO, Vec3::X];
        let mut total = 0;
        // Hold for ~0.1s at 780 RPM → interval ~0.077s → about 2 shots
        for _ in 0..20 {
            let d = fire.tick(0.01, &mut s, true, 0, aim, &m);
            total += d.len();
        }
        assert!(total >= 2, "total discharges={total}");
    }

    #[test]
    fn burst_three_and_blocks_side() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'd')).unwrap();
        fire.pay_ready(b'd');
        fire.ready_s = 0.0;
        let aim = Vec3::Z;
        let m = muzzles();
        let d0 = fire.tick(0.0, &mut s, true, 0, aim, &m);
        assert_eq!(d0.len(), 1);
        assert!(fire.blocks_weapon_side());
        // finish string
        let mut n = 1;
        for _ in 0..20 {
            fire.cooldown_s = 0.0;
            let d = fire.tick(0.001, &mut s, false, 0, aim, &m);
            n += d.len();
            if !fire.burst_active() {
                break;
            }
        }
        assert_eq!(n, 3);
        assert!(!fire.blocks_weapon_side());
    }

    #[test]
    fn fire_clears_sprint_and_taxes() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        s.apply_move(0.05, 1.0, 0.0, true);
        assert!(s.sprint_latched);
        let m = muzzles();
        let _ = fire.tick(0.0, &mut s, true, 0, Vec3::Z, &m);
        assert!(!s.sprint_latched);
        assert!(fire.sprint_fire_s > 0.1);
    }

    #[test]
    fn projectile_falls_under_gravity() {
        let mut world = ProjectileWorld::new();
        world.spawn(Projectile {
            id: 1,
            owner: 0,
            weapon: b'b',
            origin: Vec3::ZERO,
            position: Vec3::ZERO,
            velocity: Vec3::new(0.0, 0.0, 100.0),
            traveled: 0.0,
            max_range: 1000.0,
            muzzle_index: 0,
        });
        world.tick(1.0);
        let p = &world.projectiles[0];
        assert!(p.velocity.y < 0.0);
        assert!(p.position.y < 0.0);
        assert!(p.traveled > 0.0);
    }

    #[test]
    fn despawn_at_max_range() {
        let mut world = ProjectileWorld::new();
        world.spawn(Projectile {
            id: 1,
            owner: 0,
            weapon: b'b',
            origin: Vec3::ZERO,
            position: Vec3::ZERO,
            velocity: Vec3::new(0.0, 0.0, 50.0),
            traveled: 0.0,
            max_range: 10.0,
            muzzle_index: 0,
        });
        world.tick(1.0); // travels 50m > 10
        assert!(world.projectiles.is_empty());
    }

    #[test]
    fn equip_flips_to_primary_for_rifle_on_secondary() {
        let mut s = SelfState::default_loadout();
        s.active = ActiveWeapon::Secondary;
        assert_eq!(s.active_blaster(), Some(b'b'));
        equip_blaster_letter(&mut s, b'p').unwrap();
        assert_eq!(s.active, ActiveWeapon::Primary);
        assert_eq!(s.primary, Some(b'p'));
    }

    #[test]
    fn shotgun_k_spawns_six_pellets() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'k')).unwrap();
        fire.pay_ready(b'k');
        fire.ready_s = 0.0;
        let d = fire.tick(0.0, &mut s, true, 0, Vec3::Z, &muzzles());
        assert_eq!(d[0].projectiles.len(), 6);
    }

    #[test]
    fn jolt_about_grip_keeps_grip_under_pure_rotation() {
        let grip = Vec3::new(0.0, -0.14, 1.21);
        let j = JoltPose {
            pitch_rad: 0.2,
            yaw_rad: -0.1,
            back_m: 0.0,
        };
        let m = j.matrix_about_grip(grip);
        let out = m.transform_point3(grip);
        assert!(
            (out - grip).length() < 1e-5,
            "grip moved under pure rot: {out} vs {grip}"
        );
        // Mesh origin should move (orbit grip).
        let origin = m.transform_point3(Vec3::ZERO);
        assert!(origin.length() > 1e-3);
    }
}
