//! Ammo drops and walk-over take math (059). Invisible loot; corpse is separate.

use glam::Vec3;

use crate::{max_dump_rounds_for, AmmoKind, AMMO_DROP_LIFETIME_S, LOOT_TAKE_RADIUS_M};

/// Invisible ammo drop pinned at a world position (059).
#[derive(Debug, Clone, PartialEq)]
pub struct AmmoDrop {
    pub id: u64,
    pub position: Vec3,
    pub kind: AmmoKind,
    pub rounds: u16,
    pub age_s: f32,
}

impl AmmoDrop {
    pub fn new(id: u64, position: Vec3, kind: AmmoKind, rounds: u16) -> Self {
        Self {
            id,
            position,
            kind,
            rounds,
            age_s: 0.0,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.age_s += dt.max(0.0);
    }

    pub fn expired(&self) -> bool {
        self.age_s >= AMMO_DROP_LIFETIME_S || self.rounds == 0
    }

    /// How many rounds a taker with `reserve_room` may take this step.
    pub fn take_amount(&self, reserve_room: u16) -> u16 {
        self.rounds.min(reserve_room)
    }

    /// Remove up to `n` rounds; returns how many removed.
    pub fn take_rounds(&mut self, n: u16) -> u16 {
        let got = self.rounds.min(n);
        self.rounds -= got;
        got
    }
}

/// Clamp a victim-reported dump to a plausible payload (059).
pub fn clamp_dump_rounds(kind: AmmoKind, rounds: u16) -> u16 {
    rounds.min(max_dump_rounds_for(kind))
}

/// True when `pos` is within take radius of the drop.
pub fn in_take_radius(drop_pos: Vec3, pos: Vec3) -> bool {
    drop_pos.distance(pos) <= LOOT_TAKE_RADIUS_M
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_partial_leaves_rest() {
        let mut d = AmmoDrop::new(1, Vec3::ZERO, AmmoKind::LightFoam, 10);
        assert_eq!(d.take_rounds(4), 4);
        assert_eq!(d.rounds, 6);
        assert!(!d.expired());
        assert_eq!(d.take_rounds(100), 6);
        assert!(d.expired());
    }

    #[test]
    fn clamp_dump_respects_cap() {
        assert_eq!(
            clamp_dump_rounds(AmmoKind::Grenade, 999),
            max_dump_rounds_for(AmmoKind::Grenade)
        );
        assert_eq!(clamp_dump_rounds(AmmoKind::Grenade, 2), 2);
    }
}
