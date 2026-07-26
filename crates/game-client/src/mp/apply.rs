//! Pure session apply paths (roster + YouSpawned).

use game_net::{NetRole, NetVec3, PlayerId, RosterEntry};

use super::phase::{MpPhase, PendingSpawn};
use super::remotes::RemoteTable;

/// Roster + remote membership. Remotes are non-local players only.
pub fn apply_roster(
    roster: &mut Vec<RosterEntry>,
    remotes: &mut RemoteTable,
    player_id: Option<PlayerId>,
    entries: Vec<RosterEntry>,
) {
    remotes.retain(|id| {
        entries
            .iter()
            .any(|e| e.id == id && e.role == NetRole::Player && Some(e.id) != player_id)
    });
    for e in &entries {
        if Some(e.id) != player_id && e.role == NetRole::Player {
            remotes.note_joined(e.id);
        }
    }
    *roster = entries;
}

/// Local pose + phase. Roster living arrives on the next snapshot.
/// `loadout` is the staged bench choice applied on this spawn.
pub fn apply_you_spawned(
    phase: &mut MpPhase,
    spawn_requested: &mut bool,
    pending_spawn: &mut Option<PendingSpawn>,
    position: NetVec3,
    facing: f32,
    loadout: PendingSpawnLoadout,
) -> bool {
    if *phase != MpPhase::Ready || !phase.can_go(MpPhase::Living) {
        return false;
    }
    *phase = MpPhase::Living;
    *spawn_requested = false;
    *pending_spawn = Some(PendingSpawn {
        position: glam::Vec3::new(position.x, position.y, position.z),
        facing,
        primary: loadout.primary,
        secondary: loadout.secondary,
        active: loadout.active,
    });
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingSpawnLoadout {
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
    pub active: game_sim::ActiveWeapon,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        id: PlayerId,
        name: &str,
        score: u32,
        living: bool,
        role: NetRole,
        character: u8,
    ) -> RosterEntry {
        RosterEntry {
            id,
            display_name: name.into(),
            score,
            living,
            role,
            character,
        }
    }

    #[test]
    fn apply_roster_is_sole_membership_writer() {
        let mut roster = vec![entry(1, "Old", 9, true, NetRole::Player, b'a')];
        let mut remotes = RemoteTable::new();
        remotes.note_joined(1);
        remotes.note_joined(99);

        apply_roster(
            &mut roster,
            &mut remotes,
            Some(1),
            vec![
                entry(1, "Ace", 0, false, NetRole::Player, b'a'),
                entry(2, "Bee", 3, true, NetRole::Player, b'b'),
                entry(3, "Cam", 0, false, NetRole::Spectator, b'c'),
            ],
        );

        assert_eq!(roster.len(), 3);
        assert_eq!(roster[0].display_name, "Ace");
        assert!(!roster[0].living);
        assert!(roster[1].living);
        assert_eq!(roster[1].score, 3);
        assert_eq!(roster[2].role, NetRole::Spectator);
        assert_eq!(remotes.count(), 1);
        assert!(remotes.ids().any(|id| id == 2));
        assert!(!remotes.ids().any(|id| id == 1));
        assert!(!remotes.ids().any(|id| id == 3));
        assert!(!remotes.ids().any(|id| id == 99));
    }

    #[test]
    fn you_spawned_only_from_ready() {
        let mut phase = MpPhase::Lobby;
        let mut spawn_requested = true;
        let mut pending = None;
        let empty = PendingSpawnLoadout {
            primary: None,
            secondary: None,
            active: game_sim::ActiveWeapon::Primary,
        };
        assert!(!apply_you_spawned(
            &mut phase,
            &mut spawn_requested,
            &mut pending,
            NetVec3::new(1.0, 0.0, 2.0),
            0.5,
            empty,
        ));
        assert_eq!(phase, MpPhase::Lobby);
        assert!(pending.is_none());

        phase = MpPhase::Ready;
        let loadout = PendingSpawnLoadout {
            primary: Some(b'p'),
            secondary: Some(b'b'),
            active: game_sim::ActiveWeapon::Primary,
        };
        assert!(apply_you_spawned(
            &mut phase,
            &mut spawn_requested,
            &mut pending,
            NetVec3::new(1.0, 0.0, 2.0),
            0.5,
            loadout,
        ));
        assert_eq!(phase, MpPhase::Living);
        assert!(!spawn_requested);
        {
            let p = pending.as_ref().expect("pose");
            assert!((p.position.x - 1.0).abs() < 1e-5);
            assert!((p.facing - 0.5).abs() < 1e-5);
            assert_eq!(p.primary, Some(b'p'));
            assert_eq!(p.secondary, Some(b'b'));
        }

        assert!(!apply_you_spawned(
            &mut phase,
            &mut spawn_requested,
            &mut pending,
            NetVec3::new(0.0, 0.0, 0.0),
            0.0,
            loadout,
        ));
    }

    #[test]
    fn phase_helpers() {
        assert!(MpPhase::Role.in_room());
        assert!(MpPhase::Character.in_room());
        assert!(MpPhase::Ready.in_room());
        assert!(MpPhase::Spectating.in_room());
        assert!(MpPhase::Living.in_room());
        assert!(!MpPhase::Lobby.in_room());
        assert!(MpPhase::Ready.blocks_play());
        assert!(MpPhase::Ready.forces_free_cursor());
        assert!(MpPhase::Spectating.blocks_play());
        assert!(!MpPhase::Spectating.forces_free_cursor());
        assert!(!MpPhase::Living.blocks_play());
        use super::super::phase::CamIntent;
        assert_eq!(
            CamIntent::derive(MpPhase::Ready, false),
            CamIntent::Overview
        );
        assert_eq!(
            CamIntent::derive(MpPhase::Spectating, false),
            CamIntent::ProductFly
        );
    }
}
