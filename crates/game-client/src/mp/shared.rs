//! Shared WebTransport session state (Rc cell for async join + frame).

use game_net::{
    MatchView, NetAmmoDropSpawn, NetBlasterDropSpawn, NetCorpseSpawn, NetImpactHit,
    NetProjectileSpawn, NetRole, PlayerId, RosterEntry, DEFAULT_CHARACTER,
};
use game_sim::ActiveWeapon;
use web_sys::WritableStreamDefaultWriter;

use super::clock::ClockSync;
use super::phase::{MpPhase, StagedLoadout};
use super::remotes::RemoteTable;

pub use super::phase::PendingSpawn;

#[derive(Debug, Clone)]
pub struct PeerProjectileBatch {
    pub id: PlayerId,
    pub projectiles: Vec<NetProjectileSpawn>,
}

#[derive(Debug, Clone)]
pub struct PeerImpactHitBatch {
    pub hit: NetImpactHit,
}

#[derive(Debug, Clone, Copy)]
pub struct DeathAnnounceBatch {
    pub victim: PlayerId,
    pub killer: PlayerId,
}

#[derive(Debug, Clone)]
pub struct LootGrantBatch {
    pub drop_id: u64,
    pub player_id: PlayerId,
    pub ammo: u8,
    pub rounds: u16,
}

#[derive(Debug, Clone)]
pub struct BlasterGrantBatch {
    pub drop_id: u64,
    pub player_id: PlayerId,
    pub letter: u8,
    pub mag: u16,
}

pub struct FrameEffects {
    pub pending_spawn: Option<PendingSpawn>,
    pub error: Option<String>,
}

pub(crate) struct Shared {
    pub(crate) phase: MpPhase,
    pub(crate) clock: ClockSync,
    pub(crate) player_id: Option<PlayerId>,
    pub(crate) display_name: Option<String>,
    pub(crate) dgram_writer: Option<WritableStreamDefaultWriter>,
    pub(crate) reliable_writer: Option<WritableStreamDefaultWriter>,
    pub(crate) transport: Option<wasm_bindgen::JsValue>,
    pub(crate) last_error: Option<String>,
    pub(crate) probe_accum: f32,
    pub(crate) drive_accum: f32,
    pub(crate) spawn_retry_accum: f32,
    pub(crate) join_secs: f32,
    pub(crate) remotes: RemoteTable,
    pub(crate) roster: Vec<RosterEntry>,
    pub(crate) pending_projectiles: Vec<PeerProjectileBatch>,
    pub(crate) pending_hits: Vec<PeerImpactHitBatch>,
    pub(crate) pending_deaths: Vec<DeathAnnounceBatch>,
    pub(crate) pending_corpse_spawns: Vec<NetCorpseSpawn>,
    pub(crate) pending_corpse_ends: Vec<u64>,
    pub(crate) pending_drop_spawns: Vec<NetAmmoDropSpawn>,
    pub(crate) pending_drop_ends: Vec<u64>,
    pub(crate) pending_loot_grants: Vec<LootGrantBatch>,
    pub(crate) pending_blaster_drop_spawns: Vec<NetBlasterDropSpawn>,
    pub(crate) pending_blaster_drop_ends: Vec<u64>,
    pub(crate) pending_blaster_grants: Vec<BlasterGrantBatch>,
    pub(crate) pending_spawn: Option<PendingSpawn>,
    pub(crate) spawn_requested: bool,
    pub(crate) join_room: String,
    pub(crate) join_name: String,
    pub(crate) character: u8,
    pub(crate) role: NetRole,
    pub(crate) staged_primary: Option<u8>,
    pub(crate) staged_secondary: Option<u8>,
    pub(crate) staged_active: ActiveWeapon,
    pub(crate) match_view: MatchView,
    pub(crate) room_leader: bool,
    pub(crate) pick_map_sent: bool,
    pub(crate) start_match_sent: bool,
}

impl Shared {
    pub(crate) fn new() -> Self {
        Self {
            phase: MpPhase::Lobby,
            clock: ClockSync::new(),
            player_id: None,
            display_name: None,
            dgram_writer: None,
            reliable_writer: None,
            transport: None,
            last_error: None,
            probe_accum: 0.0,
            drive_accum: 0.0,
            spawn_retry_accum: 0.0,
            join_secs: 0.0,
            remotes: RemoteTable::new(),
            roster: Vec::new(),
            pending_projectiles: Vec::new(),
            pending_hits: Vec::new(),
            pending_deaths: Vec::new(),
            pending_corpse_spawns: Vec::new(),
            pending_corpse_ends: Vec::new(),
            pending_drop_spawns: Vec::new(),
            pending_drop_ends: Vec::new(),
            pending_loot_grants: Vec::new(),
            pending_blaster_drop_spawns: Vec::new(),
            pending_blaster_drop_ends: Vec::new(),
            pending_blaster_grants: Vec::new(),
            pending_spawn: None,
            spawn_requested: false,
            join_room: String::new(),
            join_name: String::new(),
            character: DEFAULT_CHARACTER,
            role: NetRole::Player,
            staged_primary: None,
            staged_secondary: None,
            staged_active: ActiveWeapon::Primary,
            match_view: MatchView {
                map: None,
                started: false,
            },
            room_leader: false,
            pick_map_sent: false,
            start_match_sent: false,
        }
    }

    pub(crate) fn reset_session(&mut self) {
        self.phase = MpPhase::Lobby;
        self.clock.clear();
        self.player_id = None;
        self.display_name = None;
        self.dgram_writer = None;
        self.reliable_writer = None;
        self.transport = None;
        self.probe_accum = 0.0;
        self.drive_accum = 0.0;
        self.spawn_retry_accum = 0.0;
        self.join_secs = 0.0;
        self.remotes.clear();
        self.roster.clear();
        self.pending_projectiles.clear();
        self.pending_hits.clear();
        self.pending_deaths.clear();
        self.pending_corpse_spawns.clear();
        self.pending_corpse_ends.clear();
        self.pending_drop_spawns.clear();
        self.pending_drop_ends.clear();
        self.pending_loot_grants.clear();
        self.pending_spawn = None;
        self.spawn_requested = false;
        self.character = DEFAULT_CHARACTER;
        self.role = NetRole::Player;
        self.staged_primary = None;
        self.staged_secondary = None;
        self.staged_active = ActiveWeapon::Primary;
        self.match_view = MatchView {
            map: None,
            started: false,
        };
        self.room_leader = false;
        self.pick_map_sent = false;
        self.start_match_sent = false;
    }

    pub(crate) fn staged_loadout(&self) -> StagedLoadout {
        StagedLoadout {
            primary: self.staged_primary,
            secondary: self.staged_secondary,
            active: self.staged_active,
        }
    }

    /// Roster is kit/role authority; product phase stays client-side.
    pub(crate) fn reconcile_self_from_roster(&mut self) {
        let Some(id) = self.player_id else {
            return;
        };
        let Some(me) = self.roster.iter().find(|e| e.id == id) else {
            return;
        };
        self.character = me.character;
        self.role = me.role;
        self.room_leader = me.room_leader;
    }

    /// Align local setup flags with server match truth (064).
    pub(crate) fn reconcile_match_setup(&mut self) {
        if self.match_view.map.is_some() {
            self.pick_map_sent = true;
        }
        if self.match_view.started {
            self.start_match_sent = true;
        }
    }
}

pub(crate) fn client_now_secs() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now() / 1000.0)
        .unwrap_or(0.0)
}
