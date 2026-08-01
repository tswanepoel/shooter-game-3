//! Present-side impact application (local self + remote health bookkeeping).

use std::collections::HashMap;

use game_net::PlayerId;
use game_sim::{impact_damage, HitBodyPart, PlayerHealth, SelfState, HEALTH_MAX};

/// Apply impact in this present. Returns damage applied to local self (0 otherwise).
/// Solo / non-room only for lethal outcomes — MP death is [`crate::mp`] DeathAnnounce (080).
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

/// Speculative hurt FX only — never changes health or alive (080).
pub(crate) fn flinch_impulse_damage(
    ammo: game_sim::AmmoKind,
    speed: f32,
    part: HitBodyPart,
) -> f32 {
    impact_damage(ammo, speed, part).max(0.0)
}

/// Server death call: force local figure dead for dumps / bench.
pub(crate) fn apply_server_death_local(
    local_id: PlayerId,
    self_state: &mut SelfState,
    health_by_id: &mut HashMap<PlayerId, PlayerHealth>,
) {
    if self_state.alive {
        self_state.apply_damage(HEALTH_MAX);
    }
    health_by_id.insert(local_id, PlayerHealth::read_from_self(self_state));
}

/// Server death call for a remote: die pose bookkeeping (body paint is roster living).
pub(crate) fn apply_server_death_remote(
    victim: PlayerId,
    health_by_id: &mut HashMap<PlayerId, PlayerHealth>,
) {
    let entry = health_by_id
        .entry(victim)
        .or_insert_with(PlayerHealth::full);
    if entry.alive {
        entry.health = 0.0;
        entry.regen_block_s = 0.0;
        entry.alive = false;
        entry.die_age_s = 0.0;
    }
}
