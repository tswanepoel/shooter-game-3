//! Shared WebTransport session state (Rc cell for async join + frame).

use game_net::{
    NetImpactHit, NetProjectileSpawn, NetRole, PlayerId, RosterEntry, DEFAULT_CHARACTER,
    DEFAULT_ROOM_CODE as ROOM_CODE,
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

pub struct FrameEffects {
    pub pending_spawn: Option<PendingSpawn>,
    pub error: Option<String>,
    pub release_pointer_lock: bool,
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
    pub(crate) pending_spawn: Option<PendingSpawn>,
    pub(crate) spawn_requested: bool,
    pub(crate) join_room: String,
    pub(crate) join_name: String,
    pub(crate) character: u8,
    pub(crate) role: NetRole,
    pub(crate) staged_primary: Option<u8>,
    pub(crate) staged_secondary: Option<u8>,
    pub(crate) staged_active: ActiveWeapon,
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
            pending_spawn: None,
            spawn_requested: false,
            join_room: ROOM_CODE.into(),
            join_name: String::new(),
            character: DEFAULT_CHARACTER,
            role: NetRole::Player,
            staged_primary: None,
            staged_secondary: None,
            staged_active: ActiveWeapon::Primary,
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
        self.pending_spawn = None;
        self.spawn_requested = false;
        self.character = DEFAULT_CHARACTER;
        self.role = NetRole::Player;
        self.staged_primary = None;
        self.staged_secondary = None;
        self.staged_active = ActiveWeapon::Primary;
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
    }
}

pub(crate) fn client_now_secs() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now() / 1000.0)
        .unwrap_or(0.0)
}
