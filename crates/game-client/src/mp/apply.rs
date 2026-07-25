//! Pure session apply paths (roster + YouSpawned).

use game_net::{NetVec3, PlayerId, RosterEntry};

use super::remotes::RemoteTable;
use super::{MpPhase, PendingSpawn};

/// Sole membership / score / living writer for the client session.
pub fn apply_roster(
    roster: &mut Vec<RosterEntry>,
    remotes: &mut RemoteTable,
    player_id: Option<PlayerId>,
    entries: Vec<RosterEntry>,
) {
    let ids: std::collections::HashSet<PlayerId> = entries.iter().map(|e| e.id).collect();
    remotes.retain(|id| ids.contains(&id) || Some(id) == player_id);
    for e in &entries {
        if Some(e.id) != player_id {
            remotes.note_joined(e.id);
        }
    }
    *roster = entries;
}

/// Local pose + phase only; roster living comes from the next Roster snapshot.
pub fn apply_you_spawned(
    phase: &mut MpPhase,
    spawn_requested: &mut bool,
    pending_spawn: &mut Option<PendingSpawn>,
    position: NetVec3,
    yaw: f32,
) -> bool {
    if *phase != MpPhase::Joined {
        return false;
    }
    *phase = MpPhase::Living;
    *spawn_requested = false;
    *pending_spawn = Some(PendingSpawn {
        position: glam::Vec3::new(position.x, position.y, position.z),
        yaw,
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_roster_is_sole_membership_writer() {
        let mut roster = vec![RosterEntry {
            id: 1,
            display_name: "Old".into(),
            score: 9,
            living: true,
        }];
        let mut remotes = RemoteTable::new();
        remotes.note_joined(1);
        remotes.note_joined(99);

        apply_roster(
            &mut roster,
            &mut remotes,
            Some(1),
            vec![
                RosterEntry {
                    id: 1,
                    display_name: "Ace".into(),
                    score: 0,
                    living: false,
                },
                RosterEntry {
                    id: 2,
                    display_name: "Bee".into(),
                    score: 3,
                    living: true,
                },
            ],
        );

        assert_eq!(roster.len(), 2);
        assert_eq!(roster[0].display_name, "Ace");
        assert!(!roster[0].living);
        assert!(roster[1].living);
        assert_eq!(roster[1].score, 3);
        assert_eq!(remotes.count(), 1);
        assert!(remotes.ids().any(|id| id == 2));
        assert!(!remotes.ids().any(|id| id == 1));
        assert!(!remotes.ids().any(|id| id == 99));
    }

    #[test]
    fn you_spawned_only_from_joined() {
        let mut phase = MpPhase::Solo;
        let mut spawn_requested = true;
        let mut pending = None;
        assert!(!apply_you_spawned(
            &mut phase,
            &mut spawn_requested,
            &mut pending,
            NetVec3::new(1.0, 0.0, 2.0),
            0.5,
        ));
        assert_eq!(phase, MpPhase::Solo);
        assert!(pending.is_none());

        phase = MpPhase::Joined;
        assert!(apply_you_spawned(
            &mut phase,
            &mut spawn_requested,
            &mut pending,
            NetVec3::new(1.0, 0.0, 2.0),
            0.5,
        ));
        assert_eq!(phase, MpPhase::Living);
        assert!(!spawn_requested);
        let p = pending.expect("pose");
        assert!((p.position.x - 1.0).abs() < 1e-5);
        assert!((p.yaw - 0.5).abs() < 1e-5);

        assert!(!apply_you_spawned(
            &mut phase,
            &mut spawn_requested,
            &mut pending,
            NetVec3::new(0.0, 0.0, 0.0),
            0.0,
        ));
    }

    #[test]
    fn phase_helpers() {
        assert!(MpPhase::Joined.in_room());
        assert!(MpPhase::Living.in_room());
        assert!(!MpPhase::Solo.in_room());
        assert!(MpPhase::Joined.blocks_play());
        assert!(MpPhase::Joined.forces_free_cursor());
        assert!(!MpPhase::Living.blocks_play());
    }
}
