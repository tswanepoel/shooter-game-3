//! Fire cadence, gates, and discharge spawning.

use glam::Vec3;

use super::projectile::{Discharge, Projectile};
use super::sway::SwayState;
use crate::weapons::{
    class_sway, weapon_def, FireMode, MuzzlePolicy, WeaponDef, SPRINT_FIRE_BASE_S,
};
use crate::SelfState;

pub const SEMI_PUMP_BASE_S: f32 = 0.25;
pub const SEMI_PUMP_PER_ROUND_S: f32 = 0.12;

pub const RELOAD_MAG_S: f32 = 0.55;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PumpCue {
    Start,
    Seat,
    End,
}

#[derive(Debug, Clone)]
pub struct FireState {
    pub(crate) ready_s: f32,
    pub(crate) sprint_fire_s: f32,
    pub(crate) cooldown_s: f32,
    pub(crate) pump_s: f32,
    pump_left: u16,
    pub(crate) reload_s: f32,
    pump_start_edge: bool,
    pumped_edge: bool,
    pump_end_edge: bool,
    burst_left: u8,
    burst_pending: bool,
    fire_held: bool,
    prev_held: bool,
    alt_muzzle: u8,
    armed_letter: Option<u8>,
    next_id: u64,
    rng: u32,
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
            pump_s: 0.0,
            pump_left: 0,
            reload_s: 0.0,
            pump_start_edge: false,
            pumped_edge: false,
            pump_end_edge: false,
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

    pub fn pumping(&self) -> bool {
        self.pump_s > 0.0
    }

    pub fn loading(&self) -> bool {
        self.reload_s > 0.0 || self.pump_s > 0.0 || self.pump_left > 0
    }

    /// No-mag blasters seat via pump — R is not used (081).
    pub fn begin_reload(&mut self, self_state: &SelfState) -> Option<u8> {
        if self.loading() || self.burst_active() {
            return None;
        }
        let letter = self_state.active_blaster()?;
        let def = weapon_def(letter)?;
        if !def.has_magazine() {
            return None;
        }
        if !self_state.can_reload() {
            return None;
        }
        self.reload_s = RELOAD_MAG_S;
        Some(letter)
    }

    pub fn take_pump_cues(&mut self) -> Vec<PumpCue> {
        let mut out = Vec::new();
        if self.pump_start_edge {
            self.pump_start_edge = false;
            out.push(PumpCue::Start);
        }
        if self.pumped_edge {
            self.pumped_edge = false;
            out.push(PumpCue::Seat);
        }
        if self.pump_end_edge {
            self.pump_end_edge = false;
            out.push(PumpCue::End);
        }
        out
    }

    pub fn fire_continues(&self) -> bool {
        self.fire_held || self.burst_left > 0
    }

    pub fn blocks_weapon_side(&self) -> bool {
        self.burst_active()
    }

    pub fn add_hit_impulse(&mut self, self_state: &mut SelfState, damage: f32) {
        let yaw_sign = if self.next_rand01() < 0.5 { -1.0 } else { 1.0 };
        self_state.apply_hit_impulse(damage, yaw_sign);
    }

    pub fn pay_ready(&mut self, letter: u8) {
        if let Some(def) = weapon_def(letter) {
            self.ready_s = def.t_ready;
            self.armed_letter = Some(letter);
            self.burst_left = 0;
            self.burst_pending = false;
            self.cooldown_s = 0.0;
            self.pump_s = 0.0;
            self.pump_left = 0;
            self.pump_start_edge = false;
            self.pumped_edge = false;
            self.pump_end_edge = false;
            self.reload_s = 0.0;
            self.alt_muzzle = 0;
        }
    }

    pub fn sync_active_letter(&mut self, letter: Option<u8>) {
        if letter != self.armed_letter {
            match letter {
                Some(l) => self.pay_ready(l),
                None => {
                    self.armed_letter = None;
                    self.ready_s = 0.0;
                    self.burst_left = 0;
                    self.burst_pending = false;
                    self.reload_s = 0.0;
                    self.pump_s = 0.0;
                    self.pump_left = 0;
                    self.pump_start_edge = false;
                    self.pumped_edge = false;
                    self.pump_end_edge = false;
                }
            }
        }
    }

    pub fn on_sprint_cleared_by_fire(&mut self, letter: u8) {
        let t_ready = weapon_def(letter).map(|d| d.t_ready).unwrap_or(0.0);
        self.sprint_fire_s = SPRINT_FIRE_BASE_S + t_ready;
    }

    fn gates_clear(&self) -> bool {
        self.ready_s <= 0.0
            && self.sprint_fire_s <= 0.0
            && self.cooldown_s <= 0.0
            && !self.loading()
    }

    fn next_rand01(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32) / (u32::MAX as f32)
    }

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
            self.pump_s = 0.0;
            self.pump_left = 0;
            self.pump_start_edge = false;
            self.pumped_edge = false;
            self.pump_end_edge = false;
            self.reload_s = 0.0;
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
            self.pump_s = 0.0;
            self.pump_left = 0;
            self.pump_start_edge = false;
            self.pumped_edge = false;
            self.pump_end_edge = false;
            self.reload_s = 0.0;
            return Vec::new();
        };
        let Some(def) = weapon_def(letter) else {
            self.sway.clear();
            self_state.set_shoulder_sway(0.0, 0.0);
            return Vec::new();
        };

        if self.reload_s > 0.0 {
            self.reload_s = (self.reload_s - dt).max(0.0);
            if self.reload_s <= 0.0 {
                self_state.try_reload();
            }
        }

        if self.pump_s > 0.0 {
            self.pump_s = (self.pump_s - dt).max(0.0);
            if self.pump_s <= 0.0 {
                let moved = if def.has_magazine() {
                    self_state.feed_chamber_from_mag(1)
                } else {
                    self_state.feed_chamber_from_reserve(1)
                };
                if moved > 0 {
                    self.pumped_edge = true;
                    self.pump_left = self.pump_left.saturating_sub(1);
                    if self.pump_left == 0 {
                        self.pump_end_edge = true;
                    }
                } else {
                    self.pump_left = 0;
                }
            }
        }

        self.ensure_seat(def, self_state);

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
        if fire_intent && self_state.is_emoting() {
            self_state.clear_emote();
        }
        let Some(weapon_line) = self_state.weapon_line() else {
            return Vec::new();
        };
        if fire_intent && self_state.sprint_latched {
            self_state.sprint_latched = false;
            if self_state.locomotion.is_sprint() {
                self_state.locomotion = crate::LocomotionMode::Walk;
            }
            self.on_sprint_cleared_by_fire(letter);
        }

        if self.burst_active() && press_edge {
            self.burst_pending = true;
        }

        let mut out = Vec::new();

        if self.burst_active() {
            if self.gates_clear() {
                if let Some(d) = self.spawn_discharge(
                    def,
                    owner,
                    look_origin,
                    weapon_line,
                    muzzle_worlds,
                    self_state,
                ) {
                    self.burst_left = self.burst_left.saturating_sub(1);
                    self.cooldown_s = def.shot_interval_s();
                    self.ensure_seat(def, self_state);
                    out.push(d);
                } else if self_state.active_chamber().unwrap_or(0) == 0
                    && !self_state.can_seat_chamber()
                {
                    self.burst_left = 0;
                    self.burst_pending = false;
                }
            }
            if self.burst_left == 0 {
                // Holding does not chain the next string — only a fresh press (081).
                let chain = self.burst_pending;
                self.burst_pending = false;
                self.ensure_seat(def, self_state);
                if chain
                    && (self_state.active_chamber().unwrap_or(0) > 0
                        || self_state.can_seat_chamber())
                {
                    self.burst_left = def.burst_count;
                }
            }
            return out;
        }

        let want = match def.mode {
            FireMode::Semi => press_edge,
            FireMode::FullAuto => fire_held,
            FireMode::Burst => press_edge,
        };

        if want && self.gates_clear() {
            match def.mode {
                FireMode::Burst => {
                    if let Some(d) = self.spawn_discharge(
                        def,
                        owner,
                        look_origin,
                        weapon_line,
                        muzzle_worlds,
                        self_state,
                    ) {
                        self.burst_left = def.burst_count.saturating_sub(1);
                        self.cooldown_s = def.shot_interval_s();
                        self.ensure_seat(def, self_state);
                        out.push(d);
                    }
                }
                FireMode::Semi | FireMode::FullAuto => {
                    if let Some(d) = self.spawn_discharge(
                        def,
                        owner,
                        look_origin,
                        weapon_line,
                        muzzle_worlds,
                        self_state,
                    ) {
                        self.cooldown_s = def.shot_interval_s();
                        self.ensure_seat(def, self_state);
                        out.push(d);
                    }
                }
            }
        }

        out
    }

    fn ensure_seat(&mut self, def: &WeaponDef, self_state: &mut SelfState) {
        if self.pump_s > 0.0 || self.reload_s > 0.0 {
            return;
        }
        let ch = self_state.active_chamber().unwrap_or(0);
        let cap = def.chamber_capacity();
        let room = cap.saturating_sub(ch);
        if room == 0 {
            self.pump_left = 0;
            return;
        }
        let pump_n = def.pump_count();
        if def.has_magazine() {
            let mag = self_state.active_mag().unwrap_or(0);
            if mag == 0 {
                self.pump_left = 0;
                return;
            }
            if pump_n == 0 {
                let _ = self_state.feed_chamber_from_mag(room);
                return;
            }
            if self.pump_left == 0 {
                self.pump_left = 1.min(room).min(mag).min(pump_n);
                self.pump_start_edge = true;
            }
            if self.pump_left > 0 {
                self.pump_s = PUMP_SEAT_S;
            }
        } else {
            let rsv = self_state.reserve.get(def.ammo());
            if rsv == 0 {
                self.pump_left = 0;
                return;
            }
            if pump_n == 0 {
                if ch == 0 {
                    let _ = self_state.feed_chamber_from_reserve(room);
                }
                return;
            }
            if self.pump_left == 0 {
                if ch != 0 {
                    return;
                }
                self.pump_left = pump_n.min(room).min(rsv);
                self.pump_start_edge = true;
            }
            if self.pump_left > 0 {
                self.pump_s = PUMP_SEAT_S;
            }
        }
    }

    fn spawn_discharge(
        &mut self,
        def: &WeaponDef,
        owner: u32,
        look_origin: Vec3,
        weapon_line: Vec3,
        muzzle_worlds: &[Vec3],
        self_state: &mut SelfState,
    ) -> Option<Discharge> {
        let weapon_line = {
            let len = weapon_line.length();
            if len < 1e-8 {
                Vec3::Z
            } else {
                weapon_line / len
            }
        };

        let has_kit_muzzles = !muzzle_worlds.is_empty();
        let mut muzzle_indices: Vec<usize> = if !has_kit_muzzles {
            vec![0]
        } else {
            match def.muzzle_policy {
                MuzzlePolicy::Single => vec![0],
                MuzzlePolicy::All => (0..muzzle_worlds.len()).collect(),
                MuzzlePolicy::Alternate => {
                    let n = muzzle_worlds.len().max(1);
                    let i = (self.alt_muzzle as usize) % n;
                    self.alt_muzzle = self.alt_muzzle.wrapping_add(1);
                    vec![i]
                }
            }
        };

        let seated = self_state.active_chamber().unwrap_or(0) as usize;
        muzzle_indices.truncate(seated);
        if muzzle_indices.is_empty() {
            return None;
        }
        if self_state.spend_chamber_rounds(muzzle_indices.len() as u16) == 0 {
            return None;
        }
        let fired_muzzles: Vec<u8> = if has_kit_muzzles {
            muzzle_indices.iter().map(|&i| i as u8).collect()
        } else {
            Vec::new()
        };

        let mut projectiles = Vec::new();
        for &mi in &muzzle_indices {
            for _ in 0..def.pellets {
                let dir =
                    scatter_direction(weapon_line, def.spread_half_deg, &mut || self.next_rand01());
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
        self_state.apply_fire_impulse(def, yaw_sign);

        Some(Discharge {
            weapon: def.letter,
            projectiles,
            fired_muzzles,
        })
    }
}

const PUMP_SEAT_S: f32 = SEMI_PUMP_BASE_S + SEMI_PUMP_PER_ROUND_S;

fn scatter_direction(axis: Vec3, half_deg: f32, rand01: &mut dyn FnMut() -> f32) -> Vec3 {
    if half_deg <= 1e-6 {
        return axis;
    }
    let half = half_deg.to_radians();
    let u = rand01();
    let v = rand01();
    let cos_max = half.cos();
    let cos_t = 1.0 - u * (1.0 - cos_max);
    let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
    let phi = v * std::f32::consts::TAU;

    let up = if axis.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let right = axis.cross(up).normalize_or_zero();
    let bitangent = right.cross(axis).normalize_or_zero();
    let dir = (axis * cos_t + right * (sin_t * phi.cos()) + bitangent * (sin_t * phi.sin()))
        .normalize_or_zero();
    if dir.length_squared() < 1e-12 {
        axis
    } else {
        dir
    }
}
