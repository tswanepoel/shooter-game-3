//! Fire cadence, gates, and discharge spawning.

use glam::Vec3;

use super::projectile::{Discharge, Projectile};
use super::sway::SwayState;
use crate::weapons::{
    class_sway, weapon_def, FireMode, MuzzlePolicy, WeaponDef, SPRINT_FIRE_BASE_S,
};
use crate::SelfState;

/// Fire cadence and gates.
#[derive(Debug, Clone)]
pub struct FireState {
    pub(crate) ready_s: f32,
    pub(crate) sprint_fire_s: f32,
    pub(crate) cooldown_s: f32,
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

    /// Held stream or unfinished fixed string.
    pub fn fire_continues(&self) -> bool {
        self.fire_held || self.burst_left > 0
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

        let fire_continues = fire_held || self.burst_left > 0;
        self_state.tick_joint_residual(dt, fire_continues);

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
            self_state.clear_fire_residual();
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
            self_state.look_yaw(),
            self_state.look_pitch(),
        );
        self_state.set_shoulder_sway(self.sway.pitch_rad, self.sway.yaw_rad);

        let press_edge = fire_held && !self.prev_held;
        self.fire_held = fire_held;
        self.prev_held = fire_held;

        let fire_intent =
            press_edge || (fire_held && def.mode == FireMode::FullAuto) || self.burst_active();
        // Clear emote before reading weapon line (holster hides the line).
        if fire_intent && self_state.is_emoting() {
            self_state.clear_emote();
        }
        let Some(aim) = self_state.weapon_line() else {
            return Vec::new();
        };
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
