//! Weapon fire gates, modes, and projectile motion (038/042/048).
//!
//! Cadence and discharge live here. Fire / hit / sway residual live on [`SelfState`].

use glam::Vec3;

use crate::weapons::{
    class_sway, weapon_def, FireMode, MuzzlePolicy, WeaponDef, WeaponSway, PROJECTILE_GRAVITY,
    SPRINT_FIRE_BASE_S,
};
use crate::{ActiveWeapon, AmmoKind, SelfState, WeaponClass};

/// One projectile in flight (anemic bag; motion rules live on [`ProjectileWorld`]).
///
/// Carries ammo identity; mass is looked up via [`crate::ammo_def`], not stored here.
/// Launch speed is set from the blaster's muzzle velocity at spawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Projectile {
    pub id: u64,
    /// Shooter id (0 in solo when not networked).
    pub owner: u32,
    pub weapon: u8,
    pub ammo: AmmoKind,
    pub origin: Vec3,
    pub position: Vec3,
    pub velocity: Vec3,
    /// Path length from origin (m).
    pub traveled: f32,
    pub max_range: f32,
    /// Flash muzzle index when present has kit muzzles.
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

/// Fire cadence and gates.
#[derive(Debug, Clone)]
pub struct FireState {
    ready_s: f32,
    sprint_fire_s: f32,
    cooldown_s: f32,
    burst_left: u8,
    burst_pending: bool,
    fire_held: bool,
    prev_held: bool,
    alt_muzzle: u8,
    armed_letter: Option<u8>,
    next_id: u64,
    rng: u32,
    /// Sway oscillator; writes fold/twist onto the figure each tick.
    sway: SwayState,
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
            sway: SwayState::new(0xA11_5A4E),
        }
    }

    pub fn burst_active(&self) -> bool {
        self.burst_left > 0
    }

    /// Weapon-side actions (sprint, wheel, equip) wait while a string runs.
    pub fn blocks_weapon_side(&self) -> bool {
        self.burst_active()
    }

    /// Hand-off a hit impulse from applied impact damage.
    pub fn add_hit_impulse(&mut self, self_state: &mut SelfState, damage: f32) {
        let yaw_sign = if self.next_rand01() < 0.5 { -1.0 } else { 1.0 };
        self_state.apply_hit_impulse(damage, yaw_sign);
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

    /// Advance timers / residual fall / sway; maybe discharge.
    ///
    /// Projectiles spawn at `look_origin` along weapon line after spread
    /// (pre-impulse for this shot). `muzzle_worlds` are flash only.
    pub fn tick(
        &mut self,
        dt: f32,
        self_state: &mut SelfState,
        fire_held: bool,
        owner: u32,
        look_origin: Vec3,
        muzzle_worlds: &[Vec3],
    ) -> Vec<Discharge> {
        let dt = dt.max(0.0);
        self.ready_s = (self.ready_s - dt).max(0.0);
        self.sprint_fire_s = (self.sprint_fire_s - dt).max(0.0);
        self.cooldown_s = (self.cooldown_s - dt).max(0.0);

        let string_active = fire_held || self.burst_left > 0;
        self_state.tick_aim_residual(dt, string_active);

        if !self_state.alive {
            self.sway.clear();
            self.fire_held = false;
            self.prev_held = false;
            self.burst_left = 0;
            self.burst_pending = false;
            return Vec::new();
        }

        let letter = self_state.active_blaster();
        self.sync_active_letter(letter);

        let Some(letter) = letter else {
            self.sway.clear();
            self_state.clear_aim_residual();
            self.fire_held = false;
            self.prev_held = false;
            self.burst_left = 0;
            self.burst_pending = false;
            return Vec::new();
        };
        let Some(def) = weapon_def(letter) else {
            self.sway.clear();
            self_state.set_shoulder_sway(0.0, 0.0);
            return Vec::new();
        };

        self.sway.advance(
            dt,
            class_sway(def.class),
            self_state.ocular_yaw,
            self_state.ocular_pitch,
        );
        self_state.set_shoulder_sway(self.sway.pitch_rad, self.sway.yaw_rad);

        let aim = self_state
            .weapon_line()
            .unwrap_or_else(|| self_state.ocular_forward());

        let press_edge = fire_held && !self.prev_held;
        self.fire_held = fire_held;
        self.prev_held = fire_held;

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
            if self.gates_clear() {
                if let Some(d) =
                    self.spawn_discharge(def, owner, look_origin, aim, muzzle_worlds, self_state)
                {
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

        if want && self.gates_clear() {
            match def.mode {
                FireMode::Burst => {
                    self.burst_left = def.burst_count;
                    if let Some(d) = self.spawn_discharge(
                        def,
                        owner,
                        look_origin,
                        aim,
                        muzzle_worlds,
                        self_state,
                    ) {
                        self.burst_left = self.burst_left.saturating_sub(1);
                        self.cooldown_s = def.shot_interval_s();
                        out.push(d);
                    }
                }
                FireMode::Semi | FireMode::FullAuto => {
                    if let Some(d) = self.spawn_discharge(
                        def,
                        owner,
                        look_origin,
                        aim,
                        muzzle_worlds,
                        self_state,
                    ) {
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
        look_origin: Vec3,
        aim: Vec3,
        muzzle_worlds: &[Vec3],
        self_state: &mut SelfState,
    ) -> Option<Discharge> {
        let aim = {
            let len = aim.length();
            if len < 1e-8 {
                Vec3::Z
            } else {
                aim / len
            }
        };

        // Multi-muzzle multiplies pellet groups (count); flash indices from kit list.
        let (muzzle_indices, fired_muzzles): (Vec<usize>, Vec<u8>) = if muzzle_worlds.is_empty() {
            (vec![0], Vec::new())
        } else {
            let idxs: Vec<usize> = match def.muzzle_policy {
                MuzzlePolicy::Single => vec![0],
                MuzzlePolicy::All => (0..muzzle_worlds.len()).collect(),
                MuzzlePolicy::Alternate => {
                    let n = muzzle_worlds.len().max(1);
                    let i = (self.alt_muzzle as usize) % n;
                    self.alt_muzzle = self.alt_muzzle.wrapping_add(1);
                    vec![i]
                }
            };
            let fired: Vec<u8> = idxs.iter().map(|&i| i as u8).collect();
            (idxs, fired)
        };

        let mut projectiles = Vec::new();
        for &mi in &muzzle_indices {
            for _ in 0..def.pellets {
                let dir = scatter_direction(aim, def.spread_half_deg, &mut || self.next_rand01());
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                projectiles.push(Projectile {
                    id,
                    owner,
                    weapon: def.letter,
                    ammo: def.ammo(),
                    origin: look_origin,
                    position: look_origin,
                    velocity: dir * def.muzzle_vel,
                    traveled: 0.0,
                    max_range: def.max_range,
                    muzzle_index: mi as u8,
                });
            }
        }

        let yaw_sign = if projectiles.first().map(|p| p.id & 1).unwrap_or(0) == 0 {
            1.0
        } else {
            -1.0
        };
        // Fire impulse after spawn so this shot uses pre-impulse weapon line.
        self_state.apply_fire_impulse(def, yaw_sign);

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
    if !state.alive {
        return Err("dead");
    }
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

/// Multi-band resting sway (breath + tremor + mean-reverting drift).
#[derive(Debug, Clone)]
struct SwayState {
    t: f32,
    pitch_rad: f32,
    yaw_rad: f32,
    drift_pitch: f32,
    drift_yaw: f32,
    /// 0 = fully damped (hard look), 1 = full resting sway.
    damp: f32,
    last_yaw: f32,
    last_pitch: f32,
    has_look: bool,
    rng: u32,
}

impl SwayState {
    fn new(rng: u32) -> Self {
        Self {
            t: 0.0,
            pitch_rad: 0.0,
            yaw_rad: 0.0,
            drift_pitch: 0.0,
            drift_yaw: 0.0,
            damp: 1.0,
            last_yaw: 0.0,
            last_pitch: 0.0,
            has_look: false,
            rng,
        }
    }

    fn clear(&mut self) {
        self.pitch_rad = 0.0;
        self.yaw_rad = 0.0;
        self.drift_pitch = 0.0;
        self.drift_yaw = 0.0;
        self.damp = 1.0;
        self.has_look = false;
    }

    fn next_gauss(&mut self) -> f32 {
        // Box–Muller from two xorshift samples.
        let u1 = self.next_rand01().max(1e-7);
        let u2 = self.next_rand01();
        (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
    }

    fn next_rand01(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32) / (u32::MAX as f32)
    }

    fn advance(&mut self, dt: f32, params: WeaponSway, look_yaw: f32, look_pitch: f32) {
        let dt = dt.max(0.0);
        if dt <= 0.0 {
            return;
        }

        // Look-rate damp: hard tracking quiets sway; still eases back in.
        let rate = if self.has_look {
            let dy = look_yaw - self.last_yaw;
            let dp = look_pitch - self.last_pitch;
            ((dy * dy + dp * dp).sqrt() / dt).max(0.0)
        } else {
            0.0
        };
        self.last_yaw = look_yaw;
        self.last_pitch = look_pitch;
        self.has_look = true;
        // ~1.2 rad/s full look → near-zero target.
        let target = 1.0 / (1.0 + (rate / 1.2).powi(2));
        let tau = if target < self.damp { 0.12 } else { 0.40 };
        let a = 1.0 - (-dt / tau).exp();
        self.damp += (target - self.damp) * a;

        self.t += dt;
        let t = self.t;
        let tau_d = params.drift_tau_s.max(1e-3);
        // Stationary std ≈ drift_amp: σ = amp * sqrt(2/τ).
        let sigma = params.drift_amp_deg.to_radians() * (2.0 / tau_d).sqrt();
        let noise_scale = sigma * dt.sqrt();
        self.drift_pitch += -self.drift_pitch / tau_d * dt + noise_scale * self.next_gauss();
        self.drift_yaw += -self.drift_yaw / tau_d * dt + noise_scale * self.next_gauss();

        let breath = params.breath_amp_deg.to_radians();
        let tremor = params.tremor_amp_deg.to_radians();
        let w_b = std::f32::consts::TAU * params.breath_hz;
        let w_t = std::f32::consts::TAU * params.tremor_hz;

        // Breath: mostly pitch; slight out-of-phase yaw.
        let breath_p = breath * (w_b * t).sin();
        let breath_y = breath * 0.28 * (w_b * 0.93 * t + 1.1).sin();
        // Tremor: soft micro-band, incommensurate axes.
        let tremor_p = tremor * (w_t * t).sin();
        let tremor_y = tremor * (w_t * 1.17 * t + 0.7).sin();

        let scale = self.damp;
        self.pitch_rad = (breath_p + tremor_p + self.drift_pitch) * scale;
        self.yaw_rad = (breath_y + tremor_y + self.drift_yaw) * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SelfState;

    fn armed_self() -> SelfState {
        SelfState::default_loadout()
    }

    fn eye() -> Vec3 {
        Vec3::new(0.0, 1.52, 0.27)
    }

    fn muzzles() -> Vec<Vec3> {
        vec![Vec3::new(0.0, 1.4, 0.4)]
    }

    #[test]
    fn dead_does_not_fire() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        s.apply_damage(crate::HEALTH_MAX);
        assert!(!s.alive);
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
        assert!(d.is_empty());
    }

    #[test]
    fn semi_fires_once_per_edge() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;

        let m = muzzles();
        // press
        let d0 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
        assert_eq!(d0.len(), 1);
        assert_eq!(d0[0].projectiles.len(), 1);
        // hold
        let d1 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
        assert!(d1.is_empty());
        // release + press after cooldown
        let _ = fire.tick(1.0, &mut s, false, 0, eye(), &m);
        fire.cooldown_s = 0.0;
        let d2 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
        assert_eq!(d2.len(), 1);
    }

    #[test]
    fn full_auto_while_held() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        // p is SMG alternate
        fire.pay_ready(b'p');
        fire.ready_s = 0.0;
        let m = vec![Vec3::ZERO, Vec3::X];
        let mut total = 0;
        // Hold for ~0.1s at 780 RPM → interval ~0.077s → about 2 shots
        for _ in 0..20 {
            let d = fire.tick(0.01, &mut s, true, 0, eye(), &m);
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
        let m = muzzles();
        let d0 = fire.tick(0.0, &mut s, true, 0, eye(), &m);
        assert_eq!(d0.len(), 1);
        assert!(fire.blocks_weapon_side());
        // finish string
        let mut n = 1;
        for _ in 0..20 {
            fire.cooldown_s = 0.0;
            let d = fire.tick(0.001, &mut s, false, 0, eye(), &m);
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
        let _ = fire.tick(0.0, &mut s, true, 0, eye(), &m);
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
            ammo: AmmoKind::LightFoam,
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
            ammo: AmmoKind::LightFoam,
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
    fn projectiles_spawn_from_look_not_muzzle() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        let barrel = muzzles();
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &barrel);
        assert_eq!(d.len(), 1);
        let p = &d[0].projectiles[0];
        assert!(
            (p.origin - eye()).length() < 1e-5,
            "combat origin is camera, got {:?}",
            p.origin
        );
        assert!(
            (p.origin - barrel[0]).length() > 0.1,
            "must not spawn at barrel"
        );
        assert_eq!(d[0].fired_muzzles, vec![0], "flash still names a muzzle");
    }

    #[test]
    fn combat_spawns_without_muzzle_fx_points() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &[]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].projectiles.len(), 1);
        assert!((d[0].projectiles[0].origin - eye()).length() < 1e-5);
        assert!(d[0].fired_muzzles.is_empty());
    }

    #[test]
    fn spawn_carries_ammo_and_blaster_muzzle_vel() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        // Pistol b → light foam, muzzle_vel from letter.
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        let def = weapon_def(b'b').unwrap();
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
        assert_eq!(d.len(), 1);
        let p = &d[0].projectiles[0];
        assert_eq!(p.ammo, AmmoKind::LightFoam);
        assert_eq!(p.weapon, b'b');
        assert!((p.velocity.length() - def.muzzle_vel).abs() < 1e-2);
        // Mass is looked up from ammo, not invented on the projectile bag.
        assert_eq!(crate::ammo_def(p.ammo).mass, crate::MASS_LIGHT_FOAM_KG);

        // Sniper e → thick foam, own muzzle speed.
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'e')).unwrap();
        fire.pay_ready(b'e');
        fire.ready_s = 0.0;
        let def_e = weapon_def(b'e').unwrap();
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
        let p = &d[0].projectiles[0];
        assert_eq!(p.ammo, AmmoKind::ThickFoam);
        assert!((p.velocity.length() - def_e.muzzle_vel).abs() < 1e-2);

        // Launcher a → grenade.
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'a')).unwrap();
        fire.pay_ready(b'a');
        fire.ready_s = 0.0;
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
        assert_eq!(d[0].projectiles[0].ammo, AmmoKind::Grenade);
    }

    #[test]
    fn shotgun_pellets_share_ammo_kind() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'k')).unwrap();
        fire.pay_ready(b'k');
        fire.ready_s = 0.0;
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
        assert_eq!(d[0].projectiles.len(), 6);
        for p in &d[0].projectiles {
            assert_eq!(p.ammo, AmmoKind::LightFoam);
        }
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
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
        assert_eq!(d[0].projectiles.len(), 6);
    }

    #[test]
    fn grip_bore_travels_with_fire_residual() {
        let mut s = armed_self();
        let def = weapon_def(b'b').unwrap();
        assert_eq!(s.grip_bore_m, 0.0);
        s.apply_fire_impulse(def, 1.0);
        assert!(s.grip_bore_m > 0.0, "bore={}", s.grip_bore_m);
        assert!(s.fire_fold_total() > 0.0);
        assert!(s.hip_fire_fold > 0.0 && s.shoulder_fire_fold > 0.0 && s.neck_fire_fold > 0.0);
        let bore_after = s.grip_bore_m;
        for _ in 0..120 {
            s.tick_aim_residual(1.0 / 60.0, false);
        }
        assert!(
            s.grip_bore_m < bore_after * 0.05,
            "bore did not fall: {}",
            s.grip_bore_m
        );
    }

    #[test]
    fn fire_adds_body_residual_and_settles() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        let m = muzzles();
        assert_eq!(s.fire_fold_total(), 0.0);
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &m);
        assert_eq!(d.len(), 1);
        let fold_after = s.fire_fold_total();
        assert!(fold_after > 0.0, "fire fold={fold_after}");
        assert!(s.hip_fire_fold > 0.0 && s.neck_fire_fold > 0.0);
        for _ in 0..120 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &m);
        }
        assert!(
            s.fire_fold_total() < fold_after * 0.05,
            "fire residual did not settle: {}",
            s.fire_fold_total()
        );
    }

    #[test]
    fn full_auto_fire_residual_stacks_climb() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'c')).unwrap();
        fire.pay_ready(b'c');
        fire.ready_s = 0.0;
        let m = muzzles();
        let one = weapon_def(b'c').unwrap().kick.pitch_deg.to_radians();
        let mut shots = 0u32;
        let mut peak = 0.0f32;
        let dt = 1.0 / 60.0;
        for _ in 0..45 {
            let d = fire.tick(dt, &mut s, true, 0, eye(), &m);
            shots += d.len() as u32;
            peak = peak.max(s.fire_fold_total());
        }
        assert!(shots >= 4, "expected several SMG shots, got {shots}");
        assert!(
            peak > one * 1.5,
            "fire residual should climb under spray: peak={peak} one={one} shots={shots}"
        );
    }

    #[test]
    fn fire_heat_rises_on_fire_and_recovers() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'c')).unwrap();
        fire.pay_ready(b'c');
        fire.ready_s = 0.0;
        let m = muzzles();
        let dt = 1.0 / 60.0;
        assert_eq!(s.fire_heat_weight(), 0.0);
        for _ in 0..30 {
            let _ = fire.tick(dt, &mut s, true, 0, eye(), &m);
        }
        assert!(
            s.fire_heat_weight() > 0.0 || s.fire_fall_eff_s() > 0.05,
            "expected heat under spray, w={} fall={}",
            s.fire_heat_weight(),
            s.fire_fall_eff_s()
        );
        // After spray, heat should be elevated enough that fall is slower than base.
        let fall_hot = s.fire_fall_eff_s();
        for _ in 0..60 {
            let _ = fire.tick(dt, &mut s, false, 0, eye(), &m);
        }
        assert!(
            s.fire_heat_weight() < 0.05,
            "heat should recover after string, w={}",
            s.fire_heat_weight()
        );
        assert!(fall_hot > weapon_def(b'c').unwrap().kick.settle_s);
    }

    #[test]
    fn shots_use_weapon_line_from_fire_residual() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        s.hip_fire_fold = 3f32.to_radians();
        s.shoulder_fire_fold = 7f32.to_radians();
        s.neck_fire_fold = 5f32.to_radians();
        s.compose_joints();
        s.fire_fall_s = 1000.0;
        let expected = s.weapon_line().expect("armed");
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
        let vel = d[0].projectiles[0].velocity.normalize();
        assert!(
            vel.dot(expected) > 0.995,
            "vel={vel} expected≈{expected} dot={}",
            vel.dot(expected)
        );
        assert!(
            vel.y > 0.1,
            "fire residual fold should lift aim, vel.y={}",
            vel.y
        );
    }

    #[test]
    fn armed_hold_advances_sway_on_shoulder() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        assert_eq!(s.shoulder_sway_fold, 0.0);
        assert_eq!(s.shoulder_sway_twist, 0.0);
        for _ in 0..90 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &muzzles());
        }
        let mag = s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs();
        assert!(mag > 1e-5, "sway should move while armed hold, mag={mag}");
    }

    #[test]
    fn unarmed_clears_sway_and_residual() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'p')).unwrap();
        fire.pay_ready(b'p');
        fire.ready_s = 0.0;
        for _ in 0..60 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
        }
        assert!(s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs() > 0.0);
        s.set_primary(None).unwrap();
        s.set_secondary(None).unwrap();
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
        assert_eq!(s.shoulder_sway_fold, 0.0);
        assert_eq!(s.shoulder_sway_twist, 0.0);
        assert_eq!(s.fire_fold_total(), 0.0);
        assert_eq!(s.hip_fire_fold, 0.0);
        assert_eq!(s.neck_fire_fold, 0.0);
        assert!(s.weapon_line().is_none());
    }

    #[test]
    fn shots_use_weapon_line_with_sway() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        for _ in 0..120 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
        }
        assert_eq!(s.fire_fold_total(), 0.0);
        s.compose_joints();
        let expected = s.weapon_line().expect("armed");
        assert!(
            s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs() > 1e-5,
            "expected nonzero sway on shoulder"
        );
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
        let vel = d[0].projectiles[0].velocity.normalize();
        // Spread on pistol is small; should still align roughly with weapon line.
        assert!(
            vel.dot(expected) > 0.98,
            "vel={vel} expected≈{expected} dot={}",
            vel.dot(expected)
        );
    }

    #[test]
    fn hit_impulse_from_damage_and_settles() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        assert_eq!(s.hit_fold_total(), 0.0);
        let dmg = crate::impact_damage(AmmoKind::LightFoam, 400.0, crate::HitBodyPart::Torso);
        assert!(dmg > 0.0);
        fire.add_hit_impulse(&mut s, dmg);
        let fold = s.hit_fold_total();
        assert!(fold > 0.0, "hit fold={fold}");
        assert!(s.hip_hit_fold > 0.0 && s.shoulder_hit_fold > 0.0 && s.neck_hit_fold > 0.0);
        // Stronger impact → stronger residual (within cap).
        let mut s2 = armed_self();
        fire.add_hit_impulse(
            &mut s2,
            crate::impact_damage(AmmoKind::Grenade, 400.0, crate::HitBodyPart::Torso),
        );
        assert!(s2.hit_fold_total() > fold);
        // Zero damage: no impulse.
        let mut s0 = armed_self();
        fire.add_hit_impulse(&mut s0, 0.0);
        assert_eq!(s0.hit_fold_total(), 0.0);
        // Settles.
        for _ in 0..120 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &muzzles());
        }
        assert!(
            s.hit_fold_total() < fold * 0.05,
            "hit residual did not settle: {}",
            s.hit_fold_total()
        );
    }

    #[test]
    fn shots_use_weapon_line_with_hit_residual() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        fire.add_hit_impulse(
            &mut s,
            crate::impact_damage(AmmoKind::LightFoam, 400.0, crate::HitBodyPart::Torso),
        );
        s.hip_fire_fold = 0.0;
        s.shoulder_fire_fold = 0.0;
        s.shoulder_fire_twist = 0.0;
        s.neck_fire_fold = 0.0;
        s.hit_fall_s = 1000.0;
        s.compose_joints();
        let expected = s.weapon_line().expect("armed");
        assert!(s.hit_fold_total() > 0.0);
        let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
        let vel = d[0].projectiles[0].velocity.normalize();
        assert!(
            vel.dot(expected) > 0.98,
            "vel={vel} expected≈{expected} dot={}",
            vel.dot(expected)
        );
        assert!(vel.y > 0.0, "hit fold should lift aim, vel.y={}", vel.y);
    }

    #[test]
    fn sniper_sway_quieter_than_smg() {
        fn peak_sway(letter: u8) -> f32 {
            let mut fire = FireState::new();
            let mut s = armed_self();
            s.set_primary(Some(letter)).unwrap();
            fire.pay_ready(letter);
            fire.ready_s = 0.0;
            let mut peak = 0.0f32;
            for _ in 0..600 {
                let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
                let m = s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs();
                peak = peak.max(m);
            }
            peak
        }
        let sniper = peak_sway(b'e');
        let smg = peak_sway(b'p');
        assert!(
            sniper < smg * 0.85,
            "sniper peak={sniper} should be quieter than smg={smg}"
        );
    }

    #[test]
    fn look_rate_damps_sway() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        for _ in 0..90 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
        }
        let still = s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs();
        assert!(still > 1e-5);
        // Whip look hard for a stretch.
        for i in 0..30 {
            s.ocular_yaw = i as f32 * 0.4;
            s.ocular_pitch = (i as f32 * 0.05).sin() * 0.2;
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
        }
        let moving = s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs();
        assert!(
            moving < still * 0.5,
            "look-rate should damp sway: still={still} moving={moving}"
        );
    }

    #[test]
    fn unarmed_has_no_weapon_line() {
        let mut s = armed_self();
        s.set_primary(None).unwrap();
        s.set_secondary(None).unwrap();
        assert!(s.weapon_line().is_none());
        assert!(s.reticle_world(eye()).is_none());
    }
}
