//! WebTransport multiplayer session (room join, role, character, loadout, spawn — 051–053).

mod apply;
mod clock;
mod cookie;
mod drive;
mod remotes;
mod session;

pub use cookie::load_display_name_cookie;
pub use drive::{drive_to_state, state_to_drive};
pub use game_net::DEFAULT_ROOM_CODE;
pub use remotes::{RemoteKitKey, RemoteTable};

use std::cell::RefCell;
use std::rc::Rc;

use game_net::{
    encode_c2s, is_known_character, normalize_display_name, ClientToServer, NetActiveWeapon,
    NetImpactHit, NetProjectileSpawn, NetRole, NetVec3, PlayerId, RosterEntry, DEFAULT_CHARACTER,
    DEFAULT_ROOM_CODE as ROOM_CODE, TICK_HZ,
};
use game_sim::{weapon_def, ActiveWeapon, AmmoKind, ImpactHit, Projectile, SelfState, WeaponClass};
use js_sys::{Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use web_sys::WritableStreamDefaultWriter;

use clock::ClockSync;
#[cfg(feature = "debug-tools")]
use cookie::load_display_name_cookie as load_cookie;
use session::{join_session, js_error_string};

/// Resend Spawn while Ready after user confirm until YouSpawned.
const SPAWN_RETRY_SECS: f32 = 0.5;

/// Client product phase (UI/play gates). Server role/living is separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpPhase {
    Lobby,
    Connecting,
    Role,
    Character,
    /// Loadout + Spawn bench (053). First entry and post-death.
    Ready,
    Spectating,
    Living,
}

impl MpPhase {
    pub fn in_room(self) -> bool {
        matches!(
            self,
            Self::Role | Self::Character | Self::Ready | Self::Spectating | Self::Living
        )
    }

    pub fn blocks_play(self) -> bool {
        !matches!(self, Self::Living)
    }

    pub fn forces_free_cursor(self) -> bool {
        matches!(
            self,
            Self::Lobby | Self::Connecting | Self::Role | Self::Character | Self::Ready
        )
    }

    pub fn is_spectating(self) -> bool {
        self == Self::Spectating
    }

    pub fn can_go(self, to: Self) -> bool {
        use MpPhase::*;
        matches!(
            (self, to),
            (Lobby, Connecting)
                | (Connecting, Role)
                | (Connecting, Lobby)
                | (Role, Character)
                | (Role, Spectating)
                | (Role, Lobby)
                | (Character, Ready)
                | (Character, Role)
                | (Character, Spectating)
                | (Character, Lobby)
                | (Ready, Living)
                | (Ready, Spectating)
                | (Ready, Role)
                | (Ready, Lobby)
                | (Spectating, Character)
                | (Spectating, Role)
                | (Spectating, Lobby)
                | (Living, Ready)
                | (Living, Spectating)
                | (Living, Lobby)
        )
    }
}

/// Per-frame camera intent (product phase + optional debug F8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CamIntent {
    Overview,
    ProductFly,
    DebugFly,
    Mounted,
}

impl CamIntent {
    pub fn derive(phase: MpPhase, debug_fly_wanted: bool) -> Self {
        match phase {
            MpPhase::Spectating => Self::ProductFly,
            MpPhase::Living if debug_fly_wanted => Self::DebugFly,
            MpPhase::Living => Self::Mounted,
            _ => Self::Overview,
        }
    }

    pub fn is_fly(self) -> bool {
        matches!(self, Self::ProductFly | Self::DebugFly)
    }
}

#[derive(Debug, Clone)]
pub struct PeerProjectileBatch {
    pub id: PlayerId,
    pub projectiles: Vec<NetProjectileSpawn>,
}

#[derive(Debug, Clone)]
pub struct PeerImpactHitBatch {
    pub hit: NetImpactHit,
}

#[derive(Debug, Clone)]
pub struct PendingSpawn {
    pub position: glam::Vec3,
    pub yaw: f32,
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
    pub active: ActiveWeapon,
}

/// Staged bench loadout (empty until player chooses; no defaults).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagedLoadout {
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
    pub active: ActiveWeapon,
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
    fn new() -> Self {
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

    fn staged_loadout(&self) -> StagedLoadout {
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

pub struct MpClient {
    shared: Rc<RefCell<Shared>>,
}

impl MpClient {
    pub fn new() -> Self {
        Self {
            shared: Rc::new(RefCell::new(Shared::new())),
        }
    }

    pub fn phase(&self) -> MpPhase {
        self.shared.borrow().phase
    }

    pub fn in_room(&self) -> bool {
        self.phase().in_room()
    }

    pub fn is_living(&self) -> bool {
        self.phase() == MpPhase::Living
    }

    pub fn is_connecting(&self) -> bool {
        self.phase() == MpPhase::Connecting
    }

    pub fn is_spectating(&self) -> bool {
        self.phase().is_spectating()
    }

    pub fn blocks_play(&self) -> bool {
        self.phase().blocks_play()
    }

    pub fn character(&self) -> u8 {
        self.shared.borrow().character
    }

    pub fn staged_loadout(&self) -> StagedLoadout {
        self.shared.borrow().staged_loadout()
    }

    pub fn cam_intent(&self, debug_fly_wanted: bool) -> CamIntent {
        CamIntent::derive(self.phase(), debug_fly_wanted)
    }

    pub fn remotes(&self) -> std::cell::Ref<'_, RemoteTable> {
        std::cell::Ref::map(self.shared.borrow(), |s| &s.remotes)
    }

    pub fn peer_living(&self, id: PlayerId) -> bool {
        self.shared
            .borrow()
            .roster
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.living)
            .unwrap_or(false)
    }

    pub fn roster(&self) -> Vec<RosterEntry> {
        self.shared.borrow().roster.clone()
    }

    #[cfg(feature = "debug-tools")]
    pub fn status_line(&self) -> String {
        let s = self.shared.borrow();
        match s.phase {
            MpPhase::Lobby => "mp: lobby".into(),
            MpPhase::Connecting => "mp: connecting…".into(),
            MpPhase::Role => {
                let name = s.display_name.as_deref().unwrap_or("—");
                format!("mp: role name={name}")
            }
            MpPhase::Character => format!("mp: character {}", s.character as char),
            MpPhase::Ready => {
                let p = s.staged_primary.map(|c| c as char).unwrap_or('·');
                let sec = s.staged_secondary.map(|c| c as char).unwrap_or('·');
                format!(
                    "mp: loadout kit={} p={p} s={sec} act={:?}",
                    s.character as char, s.staged_active
                )
            }
            MpPhase::Spectating => "mp: spectating".into(),
            MpPhase::Living => {
                let id = s
                    .player_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "—".into());
                let tick = s
                    .clock
                    .estimated_tick(client_now_secs())
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "—".into());
                let off = s
                    .clock
                    .offset_secs()
                    .map(|o| format!("{:.1}ms", o * 1000.0))
                    .unwrap_or_else(|| "—".into());
                let delay = s
                    .clock
                    .last_delay_secs()
                    .map(|d| format!("{:.1}ms", d * 1000.0))
                    .unwrap_or_else(|| "—".into());
                let score = s
                    .player_id
                    .and_then(|id| s.roster.iter().find(|e| e.id == id).map(|e| e.score))
                    .unwrap_or(0);
                format!(
                    "mp: living id={id} tick={tick} score={score} remotes={} offset={off} delay={delay} samples={}",
                    s.remotes.count(),
                    s.clock.sample_count()
                )
            }
        }
    }

    #[cfg(feature = "debug-tools")]
    pub fn hud_tick_field(&self) -> Option<String> {
        let s = self.shared.borrow();
        if !s.phase.in_room() {
            return None;
        }
        let tick = s
            .clock
            .estimated_tick(client_now_secs())
            .map(|t| t.to_string())
            .unwrap_or_else(|| "—".into());
        Some(format!("tick {tick}"))
    }

    pub fn player_id(&self) -> Option<PlayerId> {
        self.shared.borrow().player_id
    }

    pub fn take_peer_projectiles(&mut self) -> Vec<PeerProjectileBatch> {
        std::mem::take(&mut self.shared.borrow_mut().pending_projectiles)
    }

    pub fn take_peer_hits(&mut self) -> Vec<PeerImpactHitBatch> {
        std::mem::take(&mut self.shared.borrow_mut().pending_hits)
    }

    /// Drain spawn/error/cursor side effects for the frame loop.
    pub fn drain_frame_effects(&mut self) -> FrameEffects {
        let mut s = self.shared.borrow_mut();
        FrameEffects {
            pending_spawn: s.pending_spawn.take(),
            error: s.last_error.take(),
            release_pointer_lock: s.phase.forces_free_cursor(),
        }
    }

    pub fn claim_projectiles(&self, projectiles: &[Projectile]) {
        if projectiles.is_empty() {
            return;
        }
        let s = self.shared.borrow();
        if s.phase != MpPhase::Living {
            return;
        }
        let Some(writer) = s.dgram_writer.as_ref() else {
            return;
        };
        let tick = s.clock.estimated_tick(client_now_secs()).unwrap_or(0);
        let spawns: Vec<NetProjectileSpawn> = projectiles
            .iter()
            .map(|p| NetProjectileSpawn {
                id: p.id,
                weapon: p.weapon,
                origin: NetVec3::new(p.origin.x, p.origin.y, p.origin.z),
                velocity: NetVec3::new(p.velocity.x, p.velocity.y, p.velocity.z),
                muzzle_index: p.muzzle_index,
            })
            .collect();
        let Ok(payload) = encode_c2s(&ClientToServer::ProjectileSpawn {
            tick,
            projectiles: spawns,
        }) else {
            return;
        };
        let arr = Uint8Array::from(payload.as_slice());
        let _ = writer.write_with_chunk(&arr);
    }

    pub fn claim_hits(&self, hits: &[ImpactHit]) {
        if hits.is_empty() {
            return;
        }
        let s = self.shared.borrow();
        if s.phase != MpPhase::Living {
            return;
        }
        let Some(writer) = s.dgram_writer.as_ref() else {
            return;
        };
        let tick = s.clock.estimated_tick(client_now_secs()).unwrap_or(0);
        for h in hits {
            let Some(ammo) = ammo_kind_to_wire(h.ammo) else {
                continue;
            };
            let hit = NetImpactHit {
                projectile_id: h.projectile_id,
                target: h.target_id,
                ammo,
                speed: h.speed,
                part: h.part.to_wire(),
            };
            let Ok(payload) = encode_c2s(&ClientToServer::ImpactHit { tick, hit }) else {
                continue;
            };
            let arr = Uint8Array::from(payload.as_slice());
            let _ = writer.write_with_chunk(&arr);
        }
    }

    /// Debug-console join with cookie name and default room.
    #[cfg(feature = "debug-tools")]
    pub fn begin_join(&self) {
        let name = load_cookie().unwrap_or_else(|| ROOM_CODE.into());
        self.begin_join_with(ROOM_CODE, &name);
    }

    pub fn begin_join_with(&self, room_code: &str, display_name: &str) {
        let mut s = self.shared.borrow_mut();
        if !s.phase.can_go(MpPhase::Connecting) {
            return;
        }
        let name = match normalize_display_name(display_name) {
            Ok(n) => n,
            Err(reason) => {
                s.last_error = Some(format!("mp: {reason}"));
                return;
            }
        };
        s.phase = MpPhase::Connecting;
        s.clock.clear();
        s.player_id = None;
        s.display_name = None;
        s.remotes.clear();
        s.roster.clear();
        s.last_error = None;
        s.spawn_requested = false;
        s.join_room = room_code.to_string();
        s.join_name = name;
        drop(s);

        let shared = Rc::clone(&self.shared);
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = join_session(Rc::clone(&shared)).await {
                let msg = js_error_string(&e);
                let mut s = shared.borrow_mut();
                s.reset_session();
                s.last_error = Some(format!("mp: join failed: {msg}"));
            }
        });
    }

    pub fn choose_play(&self) {
        let mut s = self.shared.borrow_mut();
        if !s.phase.can_go(MpPhase::Character) {
            return;
        }
        s.role = NetRole::Player;
        s.phase = MpPhase::Character;
        s.spawn_requested = false;
        send_reliable_locked(
            &s,
            &ClientToServer::SetRole {
                role: NetRole::Player,
            },
        );
    }

    pub fn choose_spectate(&self) {
        let mut s = self.shared.borrow_mut();
        if !s.phase.can_go(MpPhase::Spectating) {
            return;
        }
        s.role = NetRole::Spectator;
        s.phase = MpPhase::Spectating;
        s.spawn_requested = false;
        send_reliable_locked(
            &s,
            &ClientToServer::SetRole {
                role: NetRole::Spectator,
            },
        );
    }

    /// UI back only — does not resend role.
    pub fn back_to_role(&self) {
        let mut s = self.shared.borrow_mut();
        if !s.phase.can_go(MpPhase::Role) {
            return;
        }
        s.phase = MpPhase::Role;
        s.spawn_requested = false;
    }

    /// Commit kit and advance to loadout bench (`SetCharacter` only). Character stays frozen after.
    pub fn confirm_character(&self, character: u8) -> Option<u8> {
        if !is_known_character(character) {
            return None;
        }
        let mut s = self.shared.borrow_mut();
        if s.phase != MpPhase::Character || !s.phase.can_go(MpPhase::Ready) {
            return None;
        }
        s.character = character;
        s.role = NetRole::Player;
        s.phase = MpPhase::Ready;
        s.spawn_requested = false;
        s.staged_primary = None;
        s.staged_secondary = None;
        s.staged_active = ActiveWeapon::Primary;
        send_reliable_locked(&s, &ClientToServer::SetCharacter { character });
        Some(character)
    }

    /// Stage primary (any known letter or empty). Bench only; cancels in-flight Spawn.
    pub fn stage_primary(&self, letter: Option<u8>) -> bool {
        if let Some(l) = letter {
            if WeaponClass::from_letter(l).is_none() {
                return false;
            }
        }
        let mut s = self.shared.borrow_mut();
        if s.phase != MpPhase::Ready {
            return false;
        }
        s.staged_primary = letter;
        s.spawn_requested = false;
        s.spawn_retry_accum = 0.0;
        true
    }

    /// Stage secondary (launcher/pistol or empty). Illegal class rejected.
    pub fn stage_secondary(&self, letter: Option<u8>) -> bool {
        if let Some(l) = letter {
            match WeaponClass::from_letter(l) {
                Some(c) if c.allowed_in_secondary() => {}
                _ => return false,
            }
        }
        let mut s = self.shared.borrow_mut();
        if s.phase != MpPhase::Ready {
            return false;
        }
        s.staged_secondary = letter;
        s.spawn_requested = false;
        s.spawn_retry_accum = 0.0;
        true
    }

    pub fn stage_active(&self, active: ActiveWeapon) {
        let mut s = self.shared.borrow_mut();
        if s.phase != MpPhase::Ready {
            return;
        }
        s.staged_active = active;
        s.spawn_requested = false;
        s.spawn_retry_accum = 0.0;
    }

    /// Death accepted → loadout bench; staged loadout defaults to what they died with.
    pub fn return_to_bench_after_death(&self, state: &SelfState) {
        let mut s = self.shared.borrow_mut();
        if s.phase != MpPhase::Living || !s.phase.can_go(MpPhase::Ready) {
            return;
        }
        s.phase = MpPhase::Ready;
        s.spawn_requested = false;
        s.spawn_retry_accum = 0.0;
        s.staged_primary = state.primary;
        s.staged_secondary = state.secondary;
        s.staged_active = state.active;
    }

    pub fn request_spawn(&self) {
        let mut s = self.shared.borrow_mut();
        if s.phase != MpPhase::Ready {
            return;
        }
        s.spawn_requested = true;
        s.spawn_retry_accum = SPAWN_RETRY_SECS;
        send_spawn_locked(&s);
    }

    pub fn leave(&self) {
        let mut s = self.shared.borrow_mut();
        if let Some(t) = s.transport.take() {
            if let Ok(close) = Reflect::get(&t, &"close".into()) {
                if let Ok(f) = close.dyn_into::<js_sys::Function>() {
                    let _ = f.call0(&t);
                }
            }
        }
        s.reset_session();
    }

    pub fn on_frame(&mut self, dt: f32, self_state: &SelfState) {
        let mut s = self.shared.borrow_mut();
        if !s.phase.in_room() {
            return;
        }
        s.join_secs += dt;
        s.probe_accum += dt;
        s.drive_accum += dt;

        let dgram = s.dgram_writer.clone();
        let living = s.phase == MpPhase::Living;
        let ready = s.phase == MpPhase::Ready;

        let mut send_spawn = false;
        if ready && s.spawn_requested {
            s.spawn_retry_accum += dt;
            if s.spawn_retry_accum >= SPAWN_RETRY_SECS {
                s.spawn_retry_accum = 0.0;
                send_spawn = true;
            }
        }

        let probe_interval = if s.join_secs < 1.0 { 0.05 } else { 0.2 };
        let send_probe = s.probe_accum >= probe_interval;
        if send_probe {
            s.probe_accum = 0.0;
        }

        let drive_interval = 1.0 / TICK_HZ as f32;
        let send_drive = living && s.drive_accum >= drive_interval;
        let drive_payload = if send_drive {
            s.drive_accum = 0.0;
            let tick = s.clock.estimated_tick(client_now_secs()).unwrap_or(0);
            let drive = state_to_drive(self_state);
            encode_c2s(&ClientToServer::DriveSample { tick, drive }).ok()
        } else {
            None
        };

        if send_spawn {
            send_spawn_locked(&s);
        }
        drop(s);

        let Some(writer) = dgram else {
            return;
        };

        if send_probe {
            let t1 = client_now_secs();
            if let Ok(payload) = encode_c2s(&ClientToServer::ClockProbe { t1 }) {
                let arr = Uint8Array::from(payload.as_slice());
                let _ = writer.write_with_chunk(&arr);
            }
        }

        if let Some(payload) = drive_payload {
            let arr = Uint8Array::from(payload.as_slice());
            let _ = writer.write_with_chunk(&arr);
        }
    }
}

impl Default for MpClient {
    fn default() -> Self {
        Self::new()
    }
}

fn send_spawn_locked(s: &Shared) {
    send_reliable_locked(
        s,
        &ClientToServer::Spawn {
            primary: s.staged_primary,
            secondary: s.staged_secondary,
            active: match s.staged_active {
                ActiveWeapon::Primary => NetActiveWeapon::Primary,
                ActiveWeapon::Secondary => NetActiveWeapon::Secondary,
            },
        },
    );
}

fn send_reliable_locked(s: &Shared, msg: &ClientToServer) {
    let Ok(payload) = encode_c2s(msg) else {
        return;
    };
    let Some(w) = s.reliable_writer.as_ref() else {
        return;
    };
    let arr = Uint8Array::from(payload.as_slice());
    let _ = w.write_with_chunk(&arr);
}

pub fn ammo_kind_from_wire(ammo: u8) -> Option<AmmoKind> {
    match ammo {
        0 => Some(AmmoKind::LightFoam),
        1 => Some(AmmoKind::ThickFoam),
        2 => Some(AmmoKind::Grenade),
        _ => None,
    }
}

fn ammo_kind_to_wire(ammo: AmmoKind) -> Option<u8> {
    Some(match ammo {
        AmmoKind::LightFoam => 0,
        AmmoKind::ThickFoam => 1,
        AmmoKind::Grenade => 2,
    })
}

pub fn net_spawn_to_projectile(owner: PlayerId, n: &NetProjectileSpawn) -> Option<Projectile> {
    let def = weapon_def(n.weapon)?;
    let origin = glam::Vec3::new(n.origin.x, n.origin.y, n.origin.z);
    let velocity = glam::Vec3::new(n.velocity.x, n.velocity.y, n.velocity.z);
    Some(Projectile {
        id: n.id,
        owner,
        weapon: n.weapon,
        ammo: def.ammo(),
        origin,
        position: origin,
        velocity,
        traveled: 0.0,
        max_range: def.max_range,
        muzzle_index: n.muzzle_index,
    })
}

pub(crate) fn client_now_secs() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now() / 1000.0)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod phase_tests {
    use super::*;

    #[test]
    fn transition_graph_core_paths() {
        assert!(MpPhase::Lobby.can_go(MpPhase::Connecting));
        assert!(MpPhase::Connecting.can_go(MpPhase::Role));
        assert!(MpPhase::Role.can_go(MpPhase::Character));
        assert!(MpPhase::Role.can_go(MpPhase::Spectating));
        assert!(MpPhase::Character.can_go(MpPhase::Ready));
        assert!(MpPhase::Ready.can_go(MpPhase::Living));
        assert!(MpPhase::Spectating.can_go(MpPhase::Character));
        assert!(MpPhase::Living.can_go(MpPhase::Spectating));
        assert!(MpPhase::Living.can_go(MpPhase::Ready));
        assert!(!MpPhase::Lobby.can_go(MpPhase::Living));
        assert!(!MpPhase::Spectating.can_go(MpPhase::Living));
        assert!(!MpPhase::Ready.can_go(MpPhase::Character));
    }

    #[test]
    fn cam_intent_derives() {
        assert_eq!(CamIntent::derive(MpPhase::Role, false), CamIntent::Overview);
        assert_eq!(
            CamIntent::derive(MpPhase::Spectating, true),
            CamIntent::ProductFly
        );
        assert_eq!(
            CamIntent::derive(MpPhase::Living, false),
            CamIntent::Mounted
        );
        assert_eq!(
            CamIntent::derive(MpPhase::Living, true),
            CamIntent::DebugFly
        );
    }
}
