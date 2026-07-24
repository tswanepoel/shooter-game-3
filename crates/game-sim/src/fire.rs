//! Weapon fire gates, modes, and projectile motion (038/042).

use glam::{Quat, Vec3};

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
    /// Round kind (042). Shared fact for this projectile's life.
    pub ammo: AmmoKind,
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

/// Fire cadence / gates, aim kick, and resting sway for one self.
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
    kick: KickPose,
    kick_settle_s: f32,
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
            kick: KickPose::default(),
            kick_settle_s: 0.08,
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

    /// Fire kick only (no resting sway).
    pub fn kick(&self) -> KickPose {
        self.kick
    }

    /// Resting sway pitch/yaw (no kick, no grip shove).
    pub fn sway(&self) -> KickPose {
        self.sway.as_pose()
    }

    /// Kick + full sway (shots and reticle).
    pub fn aim_pose(&self) -> KickPose {
        KickPose {
            pitch_rad: self.kick.pitch_rad + self.sway.pitch_rad,
            yaw_rad: self.kick.yaw_rad + self.sway.yaw_rad,
            back_m: self.kick.back_m,
        }
    }

    /// Kick full + reduced sway for the held mesh (same signal, lower gain).
    pub fn mesh_pose(&self) -> KickPose {
        const SWAY_MESH: f32 = 0.28;
        KickPose {
            pitch_rad: self.kick.pitch_rad + self.sway.pitch_rad * SWAY_MESH,
            yaw_rad: self.kick.yaw_rad + self.sway.yaw_rad * SWAY_MESH,
            back_m: self.kick.back_m,
        }
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

    /// Advance timers, kick settle, and sway; consume fire input; maybe produce discharges.
    ///
    /// Aim is look + kick + sway. Kick is added after spawn (this shot uses pre-add).
    /// `muzzle_worlds` empty → no spawn. `owner` is projectile owner id (0 solo).
    pub fn tick(
        &mut self,
        dt: f32,
        self_state: &mut SelfState,
        fire_held: bool,
        owner: u32,
        muzzle_worlds: &[Vec3],
    ) -> Vec<Discharge> {
        let dt = dt.max(0.0);
        self.ready_s = (self.ready_s - dt).max(0.0);
        self.sprint_fire_s = (self.sprint_fire_s - dt).max(0.0);
        self.cooldown_s = (self.cooldown_s - dt).max(0.0);
        self.kick.settle(dt, self.kick_settle_s);

        let letter = self_state.active_blaster();
        self.sync_active_letter(letter);

        let Some(letter) = letter else {
            self.sway.clear();
            self.fire_held = false;
            self.prev_held = false;
            self.burst_left = 0;
            self.burst_pending = false;
            return Vec::new();
        };
        let Some(def) = weapon_def(letter) else {
            self.sway.clear();
            return Vec::new();
        };

        self.sway.advance(
            dt,
            class_sway(def.class),
            self_state.ocular_yaw,
            self_state.ocular_pitch,
        );

        let aim = aim_from_self(self_state, self.aim_pose());

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
                    ammo: def.ammo(),
                    origin,
                    position: origin,
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
        self.kick.add_kick(def, yaw_sign);
        self.kick_settle_s = def.kick.settle_s.max(1e-4);

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

/// Unit aim from look + aim offset (kick and/or sway). Camera stays on look.
pub fn aim_from_self(state: &SelfState, offset: KickPose) -> Vec3 {
    let yaw = state.ocular_yaw + offset.yaw_rad;
    let pitch = (state.ocular_pitch + offset.pitch_rad)
        .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
    let cp = pitch.cos();
    Vec3::new(yaw.sin() * cp, pitch.sin(), yaw.cos() * cp)
}

/// Pitch/yaw aim offset plus optional grip shove for the held mesh.
#[derive(Debug, Clone, Copy, Default)]
pub struct KickPose {
    pub pitch_rad: f32,
    pub yaw_rad: f32,
    pub back_m: f32,
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

    fn as_pose(&self) -> KickPose {
        KickPose {
            pitch_rad: self.pitch_rad,
            yaw_rad: self.yaw_rad,
            back_m: 0.0,
        }
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

impl KickPose {
    /// `yaw_sign` is ±1 for left/right scatter.
    pub fn add_kick(&mut self, def: &WeaponDef, yaw_sign: f32) {
        let k = def.kick;
        let sign = if yaw_sign >= 0.0 { 1.0 } else { -1.0 };
        self.pitch_rad += k.pitch_deg.to_radians();
        self.yaw_rad += k.yaw_deg.to_radians() * sign;
        self.back_m += k.back_m;
        // Cap stack at ~2.5× one kick.
        self.pitch_rad = self.pitch_rad.min(k.pitch_deg.to_radians() * 2.5);
        self.yaw_rad = self
            .yaw_rad
            .clamp(-k.yaw_deg.to_radians() * 2.5, k.yaw_deg.to_radians() * 2.5);
        self.back_m = self.back_m.min(k.back_m * 2.5);
    }

    pub fn settle(&mut self, dt: f32, settle_s: f32) {
        if settle_s <= 1e-6 {
            *self = Self::default();
            return;
        }
        let t = (dt / settle_s).clamp(0.0, 1.0);
        self.pitch_rad *= 1.0 - t;
        self.yaw_rad *= 1.0 - t;
        self.back_m *= 1.0 - t;
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

    pub fn matrix(self) -> glam::Mat4 {
        let rot = Quat::from_rotation_y(self.yaw_rad) * Quat::from_rotation_x(self.pitch_rad);
        glam::Mat4::from_rotation_translation(rot, Vec3::new(0.0, 0.0, self.back_m))
    }

    /// Kick about grip G: `T(g) · K · T(−g)`. Bore is −Z so back shove is +Z local.
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

        let m = muzzles();
        // press
        let d0 = fire.tick(1.0 / 60.0, &mut s, true, 0, &m);
        assert_eq!(d0.len(), 1);
        assert_eq!(d0[0].projectiles.len(), 1);
        // hold
        let d1 = fire.tick(1.0 / 60.0, &mut s, true, 0, &m);
        assert!(d1.is_empty());
        // release + press after cooldown
        let _ = fire.tick(1.0, &mut s, false, 0, &m);
        fire.cooldown_s = 0.0;
        let d2 = fire.tick(1.0 / 60.0, &mut s, true, 0, &m);
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
            let d = fire.tick(0.01, &mut s, true, 0, &m);
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
        let d0 = fire.tick(0.0, &mut s, true, 0, &m);
        assert_eq!(d0.len(), 1);
        assert!(fire.blocks_weapon_side());
        // finish string
        let mut n = 1;
        for _ in 0..20 {
            fire.cooldown_s = 0.0;
            let d = fire.tick(0.001, &mut s, false, 0, &m);
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
        let _ = fire.tick(0.0, &mut s, true, 0, &m);
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
    fn spawn_carries_ammo_and_blaster_muzzle_vel() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        // Pistol b → light foam, muzzle_vel from letter.
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        // Zero spread via forcing letter e is wrong; use b and check speed magnitude.
        let def = weapon_def(b'b').unwrap();
        let d = fire.tick(0.0, &mut s, true, 0, &muzzles());
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
        let d = fire.tick(0.0, &mut s, true, 0, &muzzles());
        let p = &d[0].projectiles[0];
        assert_eq!(p.ammo, AmmoKind::ThickFoam);
        assert!((p.velocity.length() - def_e.muzzle_vel).abs() < 1e-2);

        // Launcher a → grenade.
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'a')).unwrap();
        fire.pay_ready(b'a');
        fire.ready_s = 0.0;
        let d = fire.tick(0.0, &mut s, true, 0, &muzzles());
        assert_eq!(d[0].projectiles[0].ammo, AmmoKind::Grenade);
    }

    #[test]
    fn shotgun_pellets_share_ammo_kind() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'k')).unwrap();
        fire.pay_ready(b'k');
        fire.ready_s = 0.0;
        let d = fire.tick(0.0, &mut s, true, 0, &muzzles());
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
        let d = fire.tick(0.0, &mut s, true, 0, &muzzles());
        assert_eq!(d[0].projectiles.len(), 6);
    }

    #[test]
    fn kick_about_grip_keeps_grip_under_pure_rotation() {
        let grip = Vec3::new(0.0, -0.14, 1.21);
        let j = KickPose {
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

    #[test]
    fn fire_adds_kick_and_settles() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        let m = muzzles();
        assert_eq!(fire.kick().pitch_rad, 0.0);
        let d = fire.tick(0.0, &mut s, true, 0, &m);
        assert_eq!(d.len(), 1);
        let pitch_after = fire.kick().pitch_rad;
        assert!(pitch_after > 0.0, "kick pitch={pitch_after}");
        // Settle over many frames without firing.
        for _ in 0..120 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, &m);
        }
        assert!(
            fire.kick().pitch_rad < pitch_after * 0.05,
            "kick did not settle: {}",
            fire.kick().pitch_rad
        );
    }

    #[test]
    fn shots_use_look_plus_kick() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        // Zero spread so velocity is pure aim.
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        fire.kick.pitch_rad = 10f32.to_radians();
        fire.kick_settle_s = 1000.0;
        let d = fire.tick(0.0, &mut s, true, 0, &muzzles());
        let vel = d[0].projectiles[0].velocity.normalize();
        let expected = aim_from_self(
            &s,
            KickPose {
                pitch_rad: 10f32.to_radians(),
                ..KickPose::default()
            },
        );
        assert!(
            vel.dot(expected) > 0.995,
            "vel={vel} expected≈{expected} dot={}",
            vel.dot(expected)
        );
        assert!(vel.y > 0.1, "kick pitch should lift aim, vel.y={}", vel.y);
    }

    #[test]
    fn armed_hold_advances_sway() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        assert_eq!(fire.sway().pitch_rad, 0.0);
        assert_eq!(fire.sway().yaw_rad, 0.0);
        for _ in 0..90 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, &muzzles());
        }
        let mag = fire.sway().pitch_rad.abs() + fire.sway().yaw_rad.abs();
        assert!(mag > 1e-5, "sway should move while armed hold, mag={mag}");
    }

    #[test]
    fn unarmed_clears_sway() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'p')).unwrap();
        fire.pay_ready(b'p');
        fire.ready_s = 0.0;
        for _ in 0..60 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, &[]);
        }
        assert!(fire.sway().pitch_rad.abs() + fire.sway().yaw_rad.abs() > 0.0);
        s.set_primary(None).unwrap();
        s.set_secondary(None).unwrap();
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, &[]);
        assert_eq!(fire.sway().pitch_rad, 0.0);
        assert_eq!(fire.sway().yaw_rad, 0.0);
    }

    #[test]
    fn shots_use_look_plus_kick_plus_sway() {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(b'b')).unwrap();
        fire.pay_ready(b'b');
        fire.ready_s = 0.0;
        // Build resting sway, freeze kick settle so it stays zero.
        for _ in 0..120 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, &[]);
        }
        fire.kick = KickPose::default();
        let pose = fire.aim_pose();
        assert!(
            pose.pitch_rad.abs() + pose.yaw_rad.abs() > 1e-5,
            "expected nonzero sway in aim_pose"
        );
        // Force zero spread weapon path: b has spread — compare direction before scatter
        // by zeroing spread effect via matching aim_pose at fire instant (dt=0 keeps sway).
        let d = fire.tick(0.0, &mut s, true, 0, &muzzles());
        let vel = d[0].projectiles[0].velocity.normalize();
        let expected = aim_from_self(&s, pose);
        // Spread on pistol is small; should still align roughly with aim_pose.
        assert!(
            vel.dot(expected) > 0.98,
            "vel={vel} expected≈{expected} dot={}",
            vel.dot(expected)
        );
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
                let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, &[]);
                let m = fire.sway().pitch_rad.abs() + fire.sway().yaw_rad.abs();
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
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, &[]);
        }
        let still = fire.sway().pitch_rad.abs() + fire.sway().yaw_rad.abs();
        assert!(still > 1e-5);
        // Whip look hard for a stretch.
        for i in 0..30 {
            s.ocular_yaw = i as f32 * 0.4;
            s.ocular_pitch = (i as f32 * 0.05).sin() * 0.2;
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, &[]);
        }
        let moving = fire.sway().pitch_rad.abs() + fire.sway().yaw_rad.abs();
        assert!(
            moving < still * 0.5,
            "look-rate should damp sway: still={still} moving={moving}"
        );
    }
}
