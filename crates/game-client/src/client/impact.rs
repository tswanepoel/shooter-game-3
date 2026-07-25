//! Present-side impact application (local self + remote health bookkeeping).

use std::collections::HashMap;

use game_net::PlayerId;
use game_sim::{impact_damage, HitBodyPart, PlayerHealth, SelfState};

/// Apply impact in this present. Returns damage applied to local self (0 otherwise).
pub(crate) fn apply_impact_in_present(
    target: PlayerId,
    ammo: game_sim::AmmoKind,
    speed: f32,
    part: HitBodyPart,
    local_id: Option<PlayerId>,
    self_state: &mut SelfState,
    health_by_id: &mut HashMap<PlayerId, PlayerHealth>,
) -> f32 {
    if local_id == Some(target) {
        if !self_state.alive {
            health_by_id.insert(target, PlayerHealth::read_from_self(self_state));
            return 0.0;
        }
        let dmg = impact_damage(ammo, speed, part);
        if dmg > 0.0 {
            self_state.apply_damage(dmg);
        }
        health_by_id.insert(target, PlayerHealth::read_from_self(self_state));
        dmg
    } else {
        let entry = health_by_id
            .entry(target)
            .or_insert_with(PlayerHealth::full);
        entry.apply_impact(ammo, speed, part);
        0.0
    }
}
