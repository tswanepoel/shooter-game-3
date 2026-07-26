//! Room roster, combat state, and spawn pose.

use std::collections::HashMap;

use game_net::{
    display_name_key, encode_s2c_frame, is_known_character, NetImpactHit, NetRole, NetVec3,
    PlayerId, RosterEntry, ServerToClient,
};
use game_sim::{impact_damage, AmmoKind, HitBodyPart, WeaponClass, HEALTH_MAX};
use tokio::sync::mpsc;
use tracing::warn;
use wtransport::Connection;

pub const SPAWN_RADIUS_M: f32 = 8.0;

#[derive(Debug, Clone, PartialEq)]
pub struct CombatState {
    pub living: bool,
    pub has_entered: bool,
    pub health: f32,
    pub score: u32,
}

impl CombatState {
    pub fn fresh() -> Self {
        Self {
            living: false,
            has_entered: false,
            health: HEALTH_MAX,
            score: 0,
        }
    }

    /// Enter or re-enter the map when not living (first spawn or post-death).
    pub fn try_enter_map(&mut self) -> bool {
        if self.living {
            return false;
        }
        self.living = true;
        self.has_entered = true;
        self.health = HEALTH_MAX;
        true
    }

    /// Apply damage while living. Returns true when this hit kills.
    pub fn receive_impact(&mut self, ammo: AmmoKind, speed: f32, part: HitBodyPart) -> bool {
        if !self.living {
            return false;
        }
        let dmg = impact_damage(ammo, speed, part);
        if dmg <= 0.0 {
            return false;
        }
        self.health = (self.health - dmg).max(0.0);
        if self.health > 0.0 {
            return false;
        }
        self.living = false;
        self.health = 0.0;
        true
    }
}

pub struct PeerEntry {
    pub connection: Connection,
    pub reliable_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub display_name: String,
    pub combat: CombatState,
    pub role: NetRole,
    pub character: u8,
}

pub struct Roster {
    peers: HashMap<PlayerId, PeerEntry>,
}

/// Lookup surface for claimed-hit resolution (roster peers or pure test maps).
trait CombatStore {
    fn living_of(&self, id: PlayerId) -> bool;
    fn combat_mut(&mut self, id: PlayerId) -> Option<&mut CombatState>;
}

impl CombatStore for HashMap<PlayerId, CombatState> {
    fn living_of(&self, id: PlayerId) -> bool {
        self.get(&id).map(|c| c.living).unwrap_or(false)
    }

    fn combat_mut(&mut self, id: PlayerId) -> Option<&mut CombatState> {
        self.get_mut(&id)
    }
}

impl CombatStore for Roster {
    fn living_of(&self, id: PlayerId) -> bool {
        self.living(id)
    }

    fn combat_mut(&mut self, id: PlayerId) -> Option<&mut CombatState> {
        self.peers.get_mut(&id).map(|p| &mut p.combat)
    }
}

/// Shared claimed-hit rules. Returns true when a kill was scored.
fn apply_impact_store(store: &mut impl CombatStore, firer: PlayerId, hit: &NetImpactHit) -> bool {
    if firer == hit.target {
        return false;
    }
    let Some(ammo) = ammo_from_wire(hit.ammo) else {
        return false;
    };
    let Some(part) = HitBodyPart::from_wire(hit.part) else {
        return false;
    };
    if !store.living_of(firer) {
        return false;
    }
    let Some(target) = store.combat_mut(hit.target) else {
        return false;
    };
    if !target.receive_impact(ammo, hit.speed, part) {
        return false;
    }
    if let Some(firer_combat) = store.combat_mut(firer) {
        firer_combat.score = firer_combat.score.saturating_add(1);
    }
    true
}

impl Roster {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    pub fn name_taken(&self, key: &str) -> bool {
        self.peers
            .values()
            .any(|p| display_name_key(&p.display_name) == key)
    }

    pub fn insert(&mut self, id: PlayerId, entry: PeerEntry) {
        self.peers.insert(id, entry);
    }

    pub fn remove(&mut self, id: PlayerId) -> bool {
        self.peers.remove(&id).is_some()
    }

    pub fn living(&self, id: PlayerId) -> bool {
        self.peers
            .get(&id)
            .map(|p| p.combat.living)
            .unwrap_or(false)
    }

    pub fn try_spawn(&mut self, id: PlayerId, primary: Option<u8>, secondary: Option<u8>) -> bool {
        let Some(peer) = self.peers.get_mut(&id) else {
            return false;
        };
        try_spawn_member(
            &peer.role,
            peer.character,
            &mut peer.combat,
            primary,
            secondary,
        )
    }

    pub fn set_role(&mut self, id: PlayerId, role: NetRole) -> bool {
        let Some(peer) = self.peers.get_mut(&id) else {
            return false;
        };
        apply_role(&mut peer.role, &mut peer.combat, role);
        true
    }

    pub fn set_character(&mut self, id: PlayerId, character: u8) -> bool {
        let Some(peer) = self.peers.get_mut(&id) else {
            return false;
        };
        apply_character(&mut peer.character, &peer.combat, character)
    }

    pub fn character(&self, id: PlayerId) -> Option<u8> {
        self.peers.get(&id).map(|p| p.character)
    }

    /// Apply a claimed hit in place. Returns true when a kill was scored.
    pub fn apply_impact(&mut self, firer: PlayerId, hit: &NetImpactHit) -> bool {
        apply_impact_store(self, firer, hit)
    }

    pub fn roster_entries(&self) -> Vec<RosterEntry> {
        let mut entries: Vec<_> = self
            .peers
            .iter()
            .map(|(&id, p)| RosterEntry {
                id,
                display_name: p.display_name.clone(),
                score: p.combat.score,
                living: p.combat.living,
                role: p.role,
                character: p.character,
            })
            .collect();
        entries.sort_by_key(|e| e.id);
        entries
    }

    pub fn roster_frame(&self, tick: u64) -> Option<Vec<u8>> {
        encode_s2c_frame(&ServerToClient::Roster {
            tick,
            entries: self.roster_entries(),
        })
        .ok()
    }

    pub fn broadcast_reliable_all(&self, bytes: &[u8]) {
        for peer in self.peers.values() {
            let _ = peer.reliable_tx.send(bytes.to_vec());
        }
    }

    pub fn send_reliable(&self, id: PlayerId, bytes: Vec<u8>) {
        if let Some(peer) = self.peers.get(&id) {
            let _ = peer.reliable_tx.send(bytes);
        }
    }

    pub fn relay_datagram(&self, except: PlayerId, bytes: &[u8]) {
        for (&id, peer) in &self.peers {
            if id == except {
                continue;
            }
            if let Err(e) = peer.connection.send_datagram(bytes) {
                warn!(peer = id, "send_datagram: {e}");
            }
        }
    }

    pub fn broadcast_roster(&self, tick: u64) {
        if let Some(bytes) = self.roster_frame(tick) {
            self.broadcast_reliable_all(&bytes);
        }
    }
}

pub fn apply_role(role: &mut NetRole, combat: &mut CombatState, new_role: NetRole) {
    *role = new_role;
    if new_role == NetRole::Spectator && combat.living {
        combat.living = false;
    }
}

pub fn apply_character(character: &mut u8, combat: &CombatState, new_character: u8) -> bool {
    if combat.living || !is_known_character(new_character) {
        return false;
    }
    *character = new_character;
    true
}

/// Class rules for staged loadout (021 / 053). Empty slots are legal.
pub fn loadout_legal(primary: Option<u8>, secondary: Option<u8>) -> bool {
    if let Some(p) = primary {
        if WeaponClass::from_letter(p).is_none() {
            return false;
        }
    }
    if let Some(s) = secondary {
        match WeaponClass::from_letter(s) {
            Some(c) if c.allowed_in_secondary() => {}
            _ => return false,
        }
    }
    true
}

pub fn try_spawn_member(
    role: &NetRole,
    character: u8,
    combat: &mut CombatState,
    primary: Option<u8>,
    secondary: Option<u8>,
) -> bool {
    if *role != NetRole::Player || !is_known_character(character) {
        return false;
    }
    if !loadout_legal(primary, secondary) {
        return false;
    }
    combat.try_enter_map()
}

pub fn ammo_from_wire(ammo: u8) -> Option<AmmoKind> {
    match ammo {
        0 => Some(AmmoKind::LightFoam),
        1 => Some(AmmoKind::ThickFoam),
        2 => Some(AmmoKind::Grenade),
        _ => None,
    }
}

/// Deterministic ground pose from tick + id (no RNG): hash → polar (x,z) + facing yaw.
pub fn spawn_pose(tick: u64, player_id: PlayerId) -> (NetVec3, f32) {
    // Splitmix-ish avalanche so nearby ticks don't cluster on the ring.
    let seed = tick
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(player_id as u64);
    let angle = unit_turn(seed, 11);
    let radius = unit01(seed, 32) * SPAWN_RADIUS_M;
    let x = angle.cos() * radius;
    let z = angle.sin() * radius;
    let yaw = unit_turn(seed, 17);
    (NetVec3::new(x, 0.0, z), yaw)
}

/// Low 32 bits of `seed >> shift` as a float in [0, 1].
fn unit01(seed: u64, shift: u32) -> f32 {
    ((seed >> shift) as u32) as f32 / (u32::MAX as f32)
}

/// Same bits mapped onto [0, τ).
fn unit_turn(seed: u64, shift: u32) -> f32 {
    unit01(seed, shift) * std::f32::consts::TAU
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_net::{display_name_key, normalize_display_name, NetImpactHit, DEFAULT_ROOM_CODE};

    #[test]
    fn enter_map_blocks_while_living_allows_after_death() {
        let mut combat = CombatState::fresh();
        combat.health = 0.0;
        assert!(combat.try_enter_map());
        assert!(combat.living && combat.has_entered);
        assert!((combat.health - HEALTH_MAX).abs() < 1e-3);
        assert!(!combat.try_enter_map());
        combat.living = false;
        combat.health = 0.0;
        assert!(combat.try_enter_map());
        assert!(combat.living);
        assert!((combat.health - HEALTH_MAX).abs() < 1e-3);
    }

    #[test]
    fn loadout_legal_class_rules() {
        assert!(loadout_legal(None, None));
        assert!(loadout_legal(Some(b'p'), Some(b'b')));
        assert!(loadout_legal(Some(b'd'), Some(b'a')));
        assert!(!loadout_legal(Some(b'z'), None));
        assert!(!loadout_legal(None, Some(b'p'))); // smg not secondary
        assert!(!loadout_legal(Some(b'p'), Some(b'd')));
    }

    #[test]
    fn normalize_and_room_policy() {
        assert_eq!(normalize_display_name("  Ace  ").unwrap(), "Ace");
        assert!(normalize_display_name("").is_err());
        assert_eq!(display_name_key("Ace"), display_name_key("ace"));
        assert_eq!(DEFAULT_ROOM_CODE, "dev");
    }

    #[test]
    fn name_taken_case_insensitive() {
        let mut names = HashMap::new();
        names.insert(1u32, "Ace".to_string());
        names.insert(2u32, "Bee".to_string());
        let taken = |key: &str| {
            names
                .values()
                .any(|n| display_name_key(n) == display_name_key(key))
        };
        assert!(taken("ace"));
        assert!(taken("ACE"));
        assert!(!taken("Cee"));
    }

    #[test]
    fn impact_awards_score_on_lethal() {
        let mut states = HashMap::new();
        states.insert(
            1,
            CombatState {
                living: true,
                has_entered: true,
                health: HEALTH_MAX,
                score: 0,
            },
        );
        states.insert(
            2,
            CombatState {
                living: true,
                has_entered: true,
                health: 1.0,
                score: 0,
            },
        );
        let hit = NetImpactHit {
            projectile_id: 1,
            target: 2,
            ammo: 0,
            speed: 400.0,
            part: 0,
        };
        assert!(apply_impact_store(&mut states, 1, &hit));
        assert!(!states[&2].living);
        assert_eq!(states[&1].score, 1);
        assert!(!apply_impact_store(&mut states, 1, &hit));
    }

    #[test]
    fn impact_ignores_self_and_dead_firer() {
        let mut states = HashMap::new();
        states.insert(
            1,
            CombatState {
                living: false,
                has_entered: true,
                health: 0.0,
                score: 0,
            },
        );
        states.insert(
            2,
            CombatState {
                living: true,
                has_entered: true,
                health: 1.0,
                score: 0,
            },
        );
        let hit = NetImpactHit {
            projectile_id: 1,
            target: 2,
            ammo: 0,
            speed: 400.0,
            part: 0,
        };
        assert!(!apply_impact_store(&mut states, 1, &hit));
        assert!(states[&2].living);

        states.get_mut(&1).unwrap().living = true;
        let self_hit = NetImpactHit {
            projectile_id: 2,
            target: 1,
            ammo: 0,
            speed: 400.0,
            part: 0,
        };
        assert!(!apply_impact_store(&mut states, 1, &self_hit));
    }

    #[test]
    fn spawn_pose_deterministic() {
        let (a, ya) = spawn_pose(10, 1);
        let (b, yb) = spawn_pose(10, 1);
        assert_eq!(a, b);
        assert_eq!(ya, yb);
        let (c, yc) = spawn_pose(11, 1);
        assert!(a != c || ya != yc);
    }

    #[test]
    fn spawn_pose_yaw_in_unit_circle() {
        for tick in [0_u64, 1, 10, 128, 180, 1_000_000, u64::MAX / 3] {
            for id in [0_u32, 1, 7, 999] {
                let (_, yaw) = spawn_pose(tick, id);
                assert!(
                    yaw.is_finite() && (0.0..std::f32::consts::TAU).contains(&yaw),
                    "tick={tick} id={id} yaw={yaw}"
                );
            }
        }
        // High tick used to cast (seed>>17) as f32 ≫ u32::MAX → kilroradian yaw (055).
        let (_, yaw) = spawn_pose(1_000_000, 1);
        assert!(yaw < std::f32::consts::TAU);
    }

    #[test]
    fn apply_role_clears_living_on_spectate() {
        let mut role = NetRole::Player;
        let mut combat = CombatState {
            living: true,
            has_entered: true,
            health: HEALTH_MAX,
            score: 0,
        };
        apply_role(&mut role, &mut combat, NetRole::Spectator);
        assert_eq!(role, NetRole::Spectator);
        assert!(!combat.living);
    }

    #[test]
    fn apply_character_rejects_living_and_unknown() {
        let mut ch = b'a';
        let living = CombatState {
            living: true,
            has_entered: true,
            health: HEALTH_MAX,
            score: 0,
        };
        assert!(!apply_character(&mut ch, &living, b'c'));
        assert_eq!(ch, b'a');

        let waiting = CombatState::fresh();
        assert!(!apply_character(&mut ch, &waiting, b'z'));
        assert!(apply_character(&mut ch, &waiting, b'c'));
        assert_eq!(ch, b'c');
    }

    #[test]
    fn try_spawn_member_gates() {
        let mut combat = CombatState::fresh();
        assert!(!try_spawn_member(
            &NetRole::Spectator,
            b'a',
            &mut combat,
            Some(b'p'),
            Some(b'b'),
        ));
        assert!(!try_spawn_member(
            &NetRole::Player,
            b'z',
            &mut combat,
            Some(b'p'),
            Some(b'b'),
        ));
        assert!(!try_spawn_member(
            &NetRole::Player,
            b'a',
            &mut combat,
            Some(b'p'),
            Some(b'd'),
        ));
        assert!(try_spawn_member(
            &NetRole::Player,
            b'a',
            &mut combat,
            Some(b'p'),
            Some(b'b'),
        ));
        assert!(combat.living && combat.has_entered);
        assert!(!try_spawn_member(
            &NetRole::Player,
            b'a',
            &mut combat,
            Some(b'p'),
            Some(b'b'),
        ));
        combat.living = false;
        assert!(try_spawn_member(
            &NetRole::Player,
            b'a',
            &mut combat,
            None,
            None,
        ));
    }
}
