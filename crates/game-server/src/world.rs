//! Authoritative world: players, session keys, fixed tick.

use std::collections::HashMap;

use game_net::{
    Hello, Input, NetSpawn, NetVec3, PlayerId, PlayerLeft, Reject, RejectReason, ServerToClient,
    SessionKey, Snapshot, Tick, Welcome, CONTENT_REV, PROTOCOL_VERSION,
};
use game_sim::SelfState;

use crate::map::player_pose;

/// Default production-ish tick rate (025 may elevate in dev).
pub const TICK_HZ: u32 = 30;

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
        player.pending_input = Some(input);
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
        let (key, issued) = self
            .players
            .get(&viewer)
            .map(|p| (p.key, p.key_issued_tick))
            .unwrap_or((0, 0));

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
