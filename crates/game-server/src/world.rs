//! Authoritative world: players, session keys, fixed tick, input land schedule (032).

use std::collections::HashMap;
use std::time::Instant;

use game_net::{
    Hello, Input, NetSpawn, NetVec3, PlayerId, PlayerLeft, Reject, RejectReason, ServerToClient,
    SessionKey, Snapshot, Tick, Welcome, CONTENT_REV, PROTOCOL_VERSION, TICK_DURATION_SECS,
};
use game_sim::SelfState;

use crate::map::player_pose;

/// Spawn half-extent on XZ (metres).
const SPAWN_HALF_EXTENT_M: f32 = 8.0;

/// Uplink EMA blend: half-life on the order of a few hundred ms of jitter.
const UPLINK_EMA_ALPHA: f32 = 0.12;
/// Reject absurd command ages (clock skew / bad stamp).
const UPLINK_SAMPLE_MAX_SECS: f64 = 2.0;
/// How fast `L_min` and published land delay may close a step change (seconds of wall time).
const LAND_SLEW_SECS: f32 = 0.35;
/// Cap personal uplink so a broken peer cannot force unbounded stall.
const UPLINK_MAX_SECS: f32 = 0.5;

struct BufferedInput {
    land_tick: Tick,
    input: Input,
}

pub struct Player {
    pub state: SelfState,
    pub key: SessionKey,
    pub key_issued_tick: Tick,
    /// Last accepted client seq (drop older/equal).
    pub last_seq: u32,
    /// Last seq applied into sim (Snapshot `ack_seq`).
    pub last_applied_seq: u32,
    /// Inputs waiting for their land tick.
    buffer: Vec<BufferedInput>,
    /// `server_secs − client_stamp ≈ offset + L` floor (clock map).
    clock_offset: Option<f64>,
    /// Smoothed uplink `L_i` (seconds).
    uplink_ema: f32,
    /// Published land delay (`L_i + T_tick`), slewed.
    land_delay_secs: f32,
}

pub struct World {
    pub tick: Tick,
    next_id: PlayerId,
    players: HashMap<PlayerId, Player>,
    /// Trivial recycled key base (MVP nonsense; still checked).
    key_nonce: u64,
    /// Monotonic server clock epoch.
    epoch: Instant,
    /// Session floor `L_min` (slewed).
    l_min_secs: f32,
}

impl World {
    pub fn new() -> Self {
        Self {
            tick: 0,
            next_id: 1,
            players: HashMap::new(),
            key_nonce: 0xC0FF_EE00_D15C_A11E,
            epoch: Instant::now(),
            l_min_secs: 0.0,
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn now_secs(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    pub fn advance_tick(&mut self, dt: f32) {
        self.tick = self.tick.wrapping_add(1);
        self.slew_land_params(dt);
        self.apply_due_inputs(dt);
        self.maybe_recycle_keys();
    }

    fn mint_key(&mut self) -> SessionKey {
        self.key_nonce = self.key_nonce.wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.key_nonce ^ (self.tick as u64).wrapping_mul(0x0100_0000_01B3)
    }

    fn random_spawn(&self, id: PlayerId) -> NetSpawn {
        // Deterministic scatter from id (no RNG crate).
        let h = id.wrapping_mul(2654435761);
        let fx = ((h & 0xFFFF) as f32 / 65535.0) * 2.0 - 1.0;
        let fz = (((h >> 16) & 0xFFFF) as f32 / 65535.0) * 2.0 - 1.0;
        let yaw = ((h >> 8) as f32 / 65535.0) * std::f32::consts::TAU;
        NetSpawn {
            position: NetVec3::new(fx * SPAWN_HALF_EXTENT_M, 0.0, fz * SPAWN_HALF_EXTENT_M),
            yaw,
        }
    }

    /// Accept Hello → Welcome, or Reject.
    pub fn try_join(&mut self, hello: &Hello) -> Result<(PlayerId, Welcome), Reject> {
        if hello.protocol != PROTOCOL_VERSION {
            return Err(Reject {
                reason: RejectReason::ProtocolMismatch,
            });
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1;
        }

        let spawn = self.random_spawn(id);
        let key = self.mint_key();
        let issued = self.tick;

        let mut state = SelfState::default_loadout();
        state.position = glam::Vec3::new(spawn.position.x, spawn.position.y, spawn.position.z);
        state.set_look(spawn.yaw, 0.0);

        let land0 = TICK_DURATION_SECS;
        self.players.insert(
            id,
            Player {
                state,
                key,
                key_issued_tick: issued,
                last_seq: 0,
                last_applied_seq: 0,
                buffer: Vec::new(),
                clock_offset: None,
                uplink_ema: 0.0,
                land_delay_secs: land0,
            },
        );

        Ok((
            id,
            Welcome {
                you: id,
                tick: self.tick,
                spawn,
                key,
                issued_tick: issued,
                content_rev: CONTENT_REV,
            },
        ))
    }

    pub fn remove_player(&mut self, id: PlayerId) -> Option<PlayerLeft> {
        self.players.remove(&id).map(|_| PlayerLeft { id })
    }

    /// Queue input when echo key matches and seq advances. Schedules land tick (032).
    pub fn queue_input(&mut self, id: PlayerId, input: Input) -> bool {
        let recv = self.now_secs();
        let tick_now = self.tick;

        let Some(player) = self.players.get_mut(&id) else {
            return false;
        };
        if input.echo_key != player.key || input.echo_issued_tick != player.key_issued_tick {
            return false;
        }
        if input.seq <= player.last_seq && player.last_seq != 0 {
            return false;
        }
        player.last_seq = input.seq;

        update_uplink(player, recv, input.intent_stamp_secs);

        let land_tick = schedule_land_tick(player, tick_now, recv, input.intent_stamp_secs);
        player.buffer.push(BufferedInput { land_tick, input });
        true
    }

    fn slew_land_params(&mut self, dt: f32) {
        let target_min = self
            .players
            .values()
            .map(|p| p.uplink_ema)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        let rate = if LAND_SLEW_SECS > 1e-6 {
            dt / LAND_SLEW_SECS
        } else {
            1.0
        };
        let rate = rate.clamp(0.0, 1.0);
        self.l_min_secs += (target_min - self.l_min_secs) * rate;

        for player in self.players.values_mut() {
            let target = (player.uplink_ema + TICK_DURATION_SECS)
                .clamp(TICK_DURATION_SECS, UPLINK_MAX_SECS + TICK_DURATION_SECS);
            player.land_delay_secs += (target - player.land_delay_secs) * rate;
        }
    }

    fn apply_due_inputs(&mut self, dt: f32) {
        let tick = self.tick;
        for player in self.players.values_mut() {
            let mut due: Vec<Input> = Vec::new();
            player.buffer.retain(|b| {
                if b.land_tick <= tick {
                    due.push(b.input.clone());
                    false
                } else {
                    true
                }
            });
            due.sort_by_key(|i| i.seq);

            if due.is_empty() {
                player.state.apply_move(
                    dt,
                    player.state.wish_forward,
                    player.state.wish_strafe,
                    false,
                );
                continue;
            }

            let last_seq = due.last().map(|i| i.seq).unwrap_or(player.last_applied_seq);
            let merged = due
                .into_iter()
                .reduce(merge_pending_input)
                .expect("due non-empty");
            player.last_applied_seq = last_seq;
            player.state.set_look(merged.look_yaw, merged.look_pitch);
            if merged.jump {
                player.state.try_jump();
            }
            if merged.weapon_cycle != 0 {
                player.state.cycle_weapon(merged.weapon_cycle);
            }
            player.state.apply_move(
                dt,
                merged.wish_forward,
                merged.wish_strafe,
                merged.sprint_tap,
            );
        }
    }

    /// Rotate session keys (MVP: every 64 ticks).
    fn maybe_recycle_keys(&mut self) {
        if !self.tick.is_multiple_of(64) || self.tick == 0 {
            return;
        }
        let mut nonce = self.key_nonce;
        for player in self.players.values_mut() {
            nonce = nonce.wrapping_add(0x9E37_79B9_7F4A_7C15);
            player.key = nonce ^ (self.tick as u64);
            player.key_issued_tick = self.tick;
        }
        self.key_nonce = nonce;
    }

    pub fn snapshot_for(&self, viewer: PlayerId) -> Snapshot {
        let (key, issued, ack_seq, land_delay, uplink) = self
            .players
            .get(&viewer)
            .map(|p| {
                (
                    p.key,
                    p.key_issued_tick,
                    p.last_applied_seq,
                    p.land_delay_secs,
                    p.uplink_ema,
                )
            })
            .unwrap_or((0, 0, 0, TICK_DURATION_SECS, 0.0));

        let you = self
            .players
            .get(&viewer)
            .map(|p| player_pose(viewer, &p.state));
        let others: Vec<_> = self
            .players
            .iter()
            .filter(|(id, _)| **id != viewer)
            .map(|(id, p)| player_pose(*id, &p.state))
            .collect();

        Snapshot {
            tick: self.tick,
            key,
            issued_tick: issued,
            ack_seq,
            land_delay_secs: land_delay,
            uplink_secs: uplink,
            l_min_secs: self.l_min_secs,
            you,
            others,
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// Build S2C for a viewer after tick.
pub fn snapshot_msg(world: &World, viewer: PlayerId) -> ServerToClient {
    ServerToClient::Snapshot(world.snapshot_for(viewer))
}

fn update_uplink(player: &mut Player, recv_secs: f64, intent_stamp: f64) {
    if !intent_stamp.is_finite() || !recv_secs.is_finite() {
        return;
    }
    let raw = recv_secs - intent_stamp;
    if !(-0.05..=UPLINK_SAMPLE_MAX_SECS).contains(&raw) {
        return;
    }

    // Map: server_time ≈ client_stamp + offset. Bootstrap offset so first L ≈ 0.
    let offset = match player.clock_offset {
        Some(o) => o,
        None => {
            player.clock_offset = Some(raw);
            player.uplink_ema = 0.0;
            return;
        }
    };

    let mut l = (raw - offset) as f32;
    if l < 0.0 {
        // Client clock ahead or improved path: pull offset down, clamp L at 0.
        player.clock_offset = Some(raw);
        l = 0.0;
    }
    l = l.clamp(0.0, UPLINK_MAX_SECS);
    player.uplink_ema += UPLINK_EMA_ALPHA * (l - player.uplink_ema);
}

/// Land tick from intent + published land delay; late → next tick.
fn schedule_land_tick(player: &Player, tick_now: Tick, recv_secs: f64, intent_stamp: f64) -> Tick {
    let offset = player.clock_offset.unwrap_or(recv_secs - intent_stamp);
    let intent_server = intent_stamp + offset;
    let land_secs = intent_server + f64::from(player.land_delay_secs);
    let remaining = land_secs - recv_secs;
    let dt = f64::from(TICK_DURATION_SECS);
    let wait_ticks = if remaining <= 0.0 {
        1u32 // late: apply on next advance
    } else {
        ((remaining / dt).ceil() as u32).max(1)
    };
    tick_now.wrapping_add(wait_ticks)
}

/// Latest continuous sample + sticky edge actions across Inputs in one land tick.
fn merge_pending_input(prev: Input, next: Input) -> Input {
    Input {
        seq: next.seq,
        echo_key: next.echo_key,
        echo_issued_tick: next.echo_issued_tick,
        intent_stamp_secs: next.intent_stamp_secs,
        wish_forward: next.wish_forward,
        wish_strafe: next.wish_strafe,
        look_yaw: next.look_yaw,
        look_pitch: next.look_pitch,
        jump: prev.jump || next.jump,
        sprint_tap: prev.sprint_tap || next.sprint_tap,
        weapon_cycle: {
            let sum = i16::from(prev.weapon_cycle) + i16::from(next.weapon_cycle);
            sum.clamp(i8::MIN as i16, i8::MAX as i16) as i8
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_net::{Hello, PROTOCOL_VERSION, TICK_HZ};

    fn join(world: &mut World) -> (PlayerId, SessionKey, Tick) {
        let (id, welcome) = world
            .try_join(&Hello {
                protocol: PROTOCOL_VERSION,
                content_rev: CONTENT_REV,
            })
            .expect("join");
        (id, welcome.key, welcome.issued_tick)
    }

    fn base_input(seq: u32, key: SessionKey, issued: Tick) -> Input {
        Input {
            seq,
            echo_key: key,
            echo_issued_tick: issued,
            intent_stamp_secs: 0.0,
            wish_forward: 0.0,
            wish_strafe: 0.0,
            look_yaw: 0.0,
            look_pitch: 0.0,
            jump: false,
            sprint_tap: false,
            weapon_cycle: 0,
        }
    }

    #[test]
    fn jump_edge_survives_merge_before_land() {
        let mut world = World::new();
        let (id, key, issued) = join(&mut world);

        let mut jump = base_input(1, key, issued);
        jump.jump = true;
        jump.intent_stamp_secs = world.now_secs();
        assert!(world.queue_input(id, jump));

        let mut quiet = base_input(2, key, issued);
        quiet.intent_stamp_secs = world.now_secs();
        assert!(world.queue_input(id, quiet));

        let buf = &world.players.get(&id).unwrap().buffer;
        assert_eq!(buf.len(), 2);

        let dt = 1.0 / TICK_HZ as f32;
        // Land is at least one tick out; advance until air.
        let mut airborne = false;
        for _ in 0..8 {
            world.advance_tick(dt);
            let loco = world.players.get(&id).unwrap().state.locomotion;
            if matches!(loco, game_sim::LocomotionMode::Air) {
                airborne = true;
                break;
            }
        }
        assert!(airborne, "merged jump should apply on land tick");
    }

    #[test]
    fn sprint_tap_and_weapon_cycle_sticky_merge() {
        let mut world = World::new();
        let (id, key, issued) = join(&mut world);

        let mut a = base_input(1, key, issued);
        a.sprint_tap = true;
        a.weapon_cycle = 1;
        a.intent_stamp_secs = world.now_secs();
        assert!(world.queue_input(id, a));

        let mut b = base_input(2, key, issued);
        b.wish_forward = 1.0;
        b.weapon_cycle = 0;
        b.intent_stamp_secs = world.now_secs();
        assert!(world.queue_input(id, b));

        // Force both to land this tick by setting land_tick <= next.
        {
            let p = world.players.get_mut(&id).unwrap();
            let t = world.tick.wrapping_add(1);
            for b in &mut p.buffer {
                b.land_tick = t;
            }
        }
        world.advance_tick(1.0 / TICK_HZ as f32);

        let st = &world.players.get(&id).unwrap().state;
        assert_eq!(st.active, game_sim::ActiveWeapon::Secondary);
        assert!((st.wish_forward - 1.0).abs() < 1e-5);
    }

    #[test]
    fn snapshot_carries_ack_seq_and_land_fields() {
        let mut world = World::new();
        let (id, key, issued) = join(&mut world);

        let snap0 = world.snapshot_for(id);
        assert_eq!(snap0.ack_seq, 0);
        assert!(snap0.land_delay_secs >= TICK_DURATION_SECS - 1e-6);

        let mut inp = base_input(7, key, issued);
        inp.intent_stamp_secs = world.now_secs();
        assert!(world.queue_input(id, inp));

        let dt = 1.0 / TICK_HZ as f32;
        for _ in 0..8 {
            world.advance_tick(dt);
            if world.players.get(&id).unwrap().last_applied_seq == 7 {
                break;
            }
        }

        let snap = world.snapshot_for(id);
        assert_eq!(snap.ack_seq, 7);
        assert!(snap.you.is_some());
        assert!(snap.land_delay_secs > 0.0);
        assert!(snap.uplink_secs >= 0.0);
        assert!(snap.l_min_secs >= 0.0);
    }

    #[test]
    fn merge_pending_or_edges() {
        let prev = Input {
            seq: 1,
            echo_key: 0,
            echo_issued_tick: 0,
            intent_stamp_secs: 0.0,
            wish_forward: 0.0,
            wish_strafe: 0.0,
            look_yaw: 0.1,
            look_pitch: 0.0,
            jump: true,
            sprint_tap: false,
            weapon_cycle: 1,
        };
        let next = Input {
            seq: 2,
            echo_key: 0,
            echo_issued_tick: 0,
            intent_stamp_secs: 0.1,
            wish_forward: 1.0,
            wish_strafe: -0.5,
            look_yaw: 0.2,
            look_pitch: -0.1,
            jump: false,
            sprint_tap: true,
            weapon_cycle: 0,
        };
        let m = merge_pending_input(prev, next);
        assert!(m.jump);
        assert!(m.sprint_tap);
        assert_eq!(m.weapon_cycle, 1);
        assert_eq!(m.wish_forward, 1.0);
        assert_eq!(m.look_yaw, 0.2);
        assert_eq!(m.seq, 2);
    }

    #[test]
    fn late_input_lands_next_tick() {
        let mut world = World::new();
        let (id, key, issued) = join(&mut world);
        // Pretend large uplink already published.
        world.players.get_mut(&id).unwrap().land_delay_secs = 0.1;
        world.players.get_mut(&id).unwrap().uplink_ema = 0.09;
        world.players.get_mut(&id).unwrap().clock_offset = Some(0.0);

        let mut inp = base_input(1, key, issued);
        // Stamp far in the past → remaining negative → late → tick+1.
        inp.intent_stamp_secs = world.now_secs() - 1.0;
        assert!(world.queue_input(id, inp));
        let land = world.players.get(&id).unwrap().buffer[0].land_tick;
        assert_eq!(land, world.tick.wrapping_add(1));
    }
}
