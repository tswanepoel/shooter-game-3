//! Authoritative world: players, session keys, fixed tick.

use std::collections::HashMap;

use game_net::{
    Hello, Input, NetSpawn, NetVec3, PlayerId, PlayerLeft, Reject, RejectReason, ServerToClient,
    SessionKey, Snapshot, Tick, Welcome, CONTENT_REV, PROTOCOL_VERSION,
};
use game_sim::SelfState;

use crate::map::player_pose;

/// Fixed server tick rate (Hz). Client render stays independent.
pub const TICK_HZ: u32 = 128;

/// Spawn half-extent on XZ (metres).
const SPAWN_HALF_EXTENT_M: f32 = 8.0;

pub struct Player {
    pub state: SelfState,
    pub key: SessionKey,
    pub key_issued_tick: Tick,
    /// Last applied client seq (drop older/equal).
    pub last_seq: u32,
    /// Latest accepted input waiting for tick apply.
    pub pending_input: Option<Input>,
}

pub struct World {
    pub tick: Tick,
    next_id: PlayerId,
    players: HashMap<PlayerId, Player>,
    /// Trivial recycled key base (MVP nonsense; still checked).
    key_nonce: u64,
}

impl World {
    pub fn new() -> Self {
        Self {
            tick: 0,
            next_id: 1,
            players: HashMap::new(),
            key_nonce: 0xC0FF_EE00_D15C_A11E,
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn advance_tick(&mut self, dt: f32) {
        self.tick = self.tick.wrapping_add(1);
        self.apply_pending_inputs(dt);
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

        self.players.insert(
            id,
            Player {
                state,
                key,
                key_issued_tick: issued,
                last_seq: 0,
                pending_input: None,
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

    /// Queue input when echo key matches and seq advances.
    ///
    /// Continuous fields (wish, look) take the latest sample. Edge actions
    /// (jump, sprint tap, weapon cycle) sticky-merge so a one-frame press is
    /// not lost when several Inputs arrive between ticks (common at low Hz).
    pub fn queue_input(&mut self, id: PlayerId, input: Input) -> bool {
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
        player.pending_input = Some(match player.pending_input.take() {
            Some(prev) => merge_pending_input(prev, input),
            None => input,
        });
        true
    }

    fn apply_pending_inputs(&mut self, dt: f32) {
        for player in self.players.values_mut() {
            let Some(input) = player.pending_input.take() else {
                // Hold last wish with zero edge actions when no new input.
                player.state.apply_move(
                    dt,
                    player.state.wish_forward,
                    player.state.wish_strafe,
                    false,
                );
                continue;
            };
            player.state.set_look(input.look_yaw, input.look_pitch);
            if input.jump {
                player.state.try_jump();
            }
            if input.weapon_cycle != 0 {
                player.state.cycle_weapon(input.weapon_cycle);
            }
            player
                .state
                .apply_move(dt, input.wish_forward, input.wish_strafe, input.sprint_tap);
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
        let (key, issued, ack_seq) = self
            .players
            .get(&viewer)
            .map(|p| (p.key, p.key_issued_tick, p.last_seq))
            .unwrap_or((0, 0, 0));

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

/// Latest continuous sample + sticky edge actions across Inputs in one tick.
fn merge_pending_input(prev: Input, next: Input) -> Input {
    Input {
        seq: next.seq,
        echo_key: next.echo_key,
        echo_issued_tick: next.echo_issued_tick,
        wish_forward: next.wish_forward,
        wish_strafe: next.wish_strafe,
        look_yaw: next.look_yaw,
        look_pitch: next.look_pitch,
        jump: prev.jump || next.jump,
        sprint_tap: prev.sprint_tap || next.sprint_tap,
        // Prefer a non-zero cycle; sum if both fire (clamped).
        weapon_cycle: {
            let sum = i16::from(prev.weapon_cycle) + i16::from(next.weapon_cycle);
            sum.clamp(i8::MIN as i16, i8::MAX as i16) as i8
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_net::{Hello, PROTOCOL_VERSION};

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
    fn jump_edge_survives_overwrite_before_tick() {
        let mut world = World::new();
        let (id, key, issued) = join(&mut world);

        let mut jump = base_input(1, key, issued);
        jump.jump = true;
        assert!(world.queue_input(id, jump));

        // Later frame in the same tick: continuous sample, no jump edge.
        let quiet = base_input(2, key, issued);
        assert!(world.queue_input(id, quiet));

        let pending = world
            .players
            .get(&id)
            .unwrap()
            .pending_input
            .as_ref()
            .unwrap();
        assert!(
            pending.jump,
            "jump must sticky-merge across Inputs before tick"
        );

        let dt = 1.0 / 30.0;
        world.advance_tick(dt);

        let loco = world.players.get(&id).unwrap().state.locomotion;
        assert!(
            matches!(loco, game_sim::LocomotionMode::Air),
            "merged jump should apply on tick"
        );
    }

    #[test]
    fn sprint_tap_and_weapon_cycle_sticky_merge() {
        let mut world = World::new();
        let (id, key, issued) = join(&mut world);

        let mut a = base_input(1, key, issued);
        a.sprint_tap = true;
        a.weapon_cycle = 1;
        assert!(world.queue_input(id, a));

        let mut b = base_input(2, key, issued);
        b.wish_forward = 1.0;
        b.weapon_cycle = 0;
        assert!(world.queue_input(id, b));

        let pending = world
            .players
            .get(&id)
            .unwrap()
            .pending_input
            .as_ref()
            .unwrap();
        assert!(pending.sprint_tap);
        assert_eq!(pending.weapon_cycle, 1);
        assert_eq!(pending.wish_forward, 1.0);
    }

    #[test]
    fn snapshot_carries_ack_seq() {
        let mut world = World::new();
        let (id, key, issued) = join(&mut world);

        let snap0 = world.snapshot_for(id);
        assert_eq!(snap0.ack_seq, 0);

        assert!(world.queue_input(id, base_input(7, key, issued)));
        world.advance_tick(1.0 / TICK_HZ as f32);

        let snap = world.snapshot_for(id);
        assert_eq!(snap.ack_seq, 7);
        assert!(snap.you.is_some());
    }

    #[test]
    fn merge_pending_or_edges() {
        let prev = Input {
            seq: 1,
            echo_key: 0,
            echo_issued_tick: 0,
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
}
