//! WebTransport multiplayer session (room join, role, character, loadout, spawn).

mod apply;
mod claims;
mod clock;
mod cookie;
mod drive;
mod lobby;
mod phase;
mod remotes;
mod send;
mod session;
mod shared;
mod tick;

pub use claims::{ammo_kind_from_wire, net_spawn_to_projectile};
pub use cookie::load_display_name_cookie;
pub use drive::drive_to_state;
pub use phase::{CamIntent, MpPhase, StagedLoadout};
pub use remotes::{RemoteKitKey, RemoteTable};
pub use shared::{
    BlasterGrantBatch, FrameEffects, LootGrantBatch, PeerImpactHitBatch, PeerProjectileBatch,
};
// Named type for `FrameEffects::pending_spawn` (phase module is private).
#[allow(unused_imports)]
pub use shared::PendingSpawn;

/// Temporary alpha join-form / console pre-fill. Nuke when room must be typed.
pub const JOIN_ROOM_PREFILL: &str = "dev";

pub(crate) use shared::{client_now_secs, Shared};

use std::cell::RefCell;
use std::rc::Rc;

use game_net::{NetAmmoDropSpawn, NetBlasterDropSpawn, NetCorpseSpawn, PlayerId, RosterEntry};
use game_sim::{ActiveWeapon, AmmoKind, ImpactHit, Projectile, SelfState};

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
        let lead = if s.room_leader { " lead" } else { "" };
        match s.phase {
            MpPhase::Lobby => "mp: lobby".into(),
            MpPhase::Connecting => "mp: connecting…".into(),
            MpPhase::Role => {
                let name = s.display_name.as_deref().unwrap_or("—");
                format!("mp: role name={name}{lead}")
            }
            MpPhase::Character => format!("mp: character {}{lead}", s.character as char),
            MpPhase::Ready => {
                let p = s.staged_primary.map(|c| c as char).unwrap_or('·');
                let sec = s.staged_secondary.map(|c| c as char).unwrap_or('·');
                format!(
                    "mp: loadout kit={} p={p} s={sec} act={:?}{lead}",
                    s.character as char, s.staged_active
                )
            }
            MpPhase::Spectating => format!("mp: spectating{lead}"),
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
                    "mp: living id={id} tick={tick} score={score} remotes={} offset={off} delay={delay} samples={}{lead}",
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

    pub fn match_started(&self) -> bool {
        self.shared.borrow().match_view.started
    }

    pub fn match_map(&self) -> Option<u8> {
        self.shared.borrow().match_view.map
    }

    pub fn take_peer_projectiles(&mut self) -> Vec<PeerProjectileBatch> {
        std::mem::take(&mut self.shared.borrow_mut().pending_projectiles)
    }

    pub fn take_peer_hits(&mut self) -> Vec<PeerImpactHitBatch> {
        std::mem::take(&mut self.shared.borrow_mut().pending_hits)
    }

    pub fn take_corpse_spawns(&mut self) -> Vec<NetCorpseSpawn> {
        std::mem::take(&mut self.shared.borrow_mut().pending_corpse_spawns)
    }

    pub fn take_corpse_ends(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.shared.borrow_mut().pending_corpse_ends)
    }

    pub fn take_drop_spawns(&mut self) -> Vec<NetAmmoDropSpawn> {
        std::mem::take(&mut self.shared.borrow_mut().pending_drop_spawns)
    }

    pub fn take_drop_ends(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.shared.borrow_mut().pending_drop_ends)
    }

    pub fn take_loot_grants(&mut self) -> Vec<LootGrantBatch> {
        std::mem::take(&mut self.shared.borrow_mut().pending_loot_grants)
    }

    pub fn take_blaster_drop_spawns(&mut self) -> Vec<NetBlasterDropSpawn> {
        std::mem::take(&mut self.shared.borrow_mut().pending_blaster_drop_spawns)
    }

    pub fn take_blaster_drop_ends(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.shared.borrow_mut().pending_blaster_drop_ends)
    }

    pub fn take_blaster_grants(&mut self) -> Vec<BlasterGrantBatch> {
        std::mem::take(&mut self.shared.borrow_mut().pending_blaster_grants)
    }

    /// Drain spawn/error side effects for the frame loop.
    pub fn drain_frame_effects(&mut self) -> FrameEffects {
        let mut s = self.shared.borrow_mut();
        FrameEffects {
            pending_spawn: s.pending_spawn.take(),
            error: s.last_error.take(),
        }
    }

    pub fn claim_projectiles(&self, projectiles: &[Projectile]) {
        claims::claim_projectiles(&self.shared, projectiles);
    }

    pub fn claim_hits(&self, hits: &[ImpactHit]) {
        claims::claim_hits(&self.shared, hits);
    }

    pub fn claim_ammo_dump(&self, kind: AmmoKind, rounds: u16, position: glam::Vec3) {
        claims::claim_ammo_dump(&self.shared, kind, rounds, position);
    }

    pub fn claim_loot(&self, drop_id: u64, position: glam::Vec3, room: u16) {
        claims::claim_loot(&self.shared, drop_id, position, room);
    }

    pub fn claim_blaster_dump(&self, letter: u8, mag: u16, position: glam::Vec3) {
        claims::claim_blaster_dump(&self.shared, letter, mag, position);
    }

    pub fn claim_blaster(&self, drop_id: u64, position: glam::Vec3) {
        claims::claim_blaster(&self.shared, drop_id, position);
    }

    /// Debug-console join with cookie name and default room.
    #[cfg(feature = "debug-tools")]
    pub fn begin_join(&self) {
        lobby::begin_join(&self.shared);
    }

    pub fn begin_join_with(&self, room_code: &str, display_name: &str) {
        lobby::begin_join_with(&self.shared, room_code, display_name);
    }

    pub fn choose_play(&self) {
        lobby::choose_play(&self.shared);
    }

    pub fn choose_spectate(&self) {
        lobby::choose_spectate(&self.shared);
    }

    /// UI back only — does not resend role.
    pub fn back_to_role(&self) {
        lobby::back_to_role(&self.shared);
    }

    /// Commit kit and advance to loadout bench (`SetCharacter` only). Character stays frozen after.
    pub fn confirm_character(&self, character: u8) -> Option<u8> {
        lobby::confirm_character(&self.shared, character)
    }

    /// Stage primary (any known letter or empty). Bench only; cancels in-flight Spawn.
    pub fn stage_primary(&self, letter: Option<u8>) -> bool {
        lobby::stage_primary(&self.shared, letter)
    }

    /// Stage secondary (launcher/pistol or empty). Illegal class rejected.
    pub fn stage_secondary(&self, letter: Option<u8>) -> bool {
        lobby::stage_secondary(&self.shared, letter)
    }

    pub fn stage_active(&self, active: ActiveWeapon) {
        lobby::stage_active(&self.shared, active);
    }

    /// Death accepted → loadout bench; staged loadout defaults to what they died with.
    pub fn return_to_bench_after_death(&self) {
        lobby::return_to_bench_after_death(&self.shared);
    }

    pub fn request_spawn(&self) {
        lobby::request_spawn(&self.shared);
    }

    pub fn leave(&self) {
        lobby::leave(&self.shared);
    }

    pub fn on_frame(&mut self, dt: f32, self_state: &SelfState) {
        tick::on_frame(&self.shared, dt, self_state);
    }
}

impl Default for MpClient {
    fn default() -> Self {
        Self::new()
    }
}
