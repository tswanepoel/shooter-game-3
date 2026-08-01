//! Client world corpses, ammo drops (059), and blaster drops (067).

use std::collections::HashMap;

use game_net::{NetAmmoDropSpawn, NetBlasterDropSpawn, NetCorpseSpawn};
use game_sim::{
    in_take_radius, look_forward_dir, looking_at_blaster, loot_look_origin, AmmoDrop, AmmoKind,
    BlasterDrop, SelfState,
};
use glam::Vec3;

#[derive(Debug, Clone)]
pub struct WorldCorpse {
    pub character: u8,
    pub position: Vec3,
    pub facing: f32,
    /// Age for die-clip hold (starts at 0 on spawn).
    pub die_age_s: f32,
}

/// Re-claim interval while still overlapping (s).
const CLAIM_RETRY_S: f32 = 0.35;

#[derive(Debug, Default)]
pub struct WorldLoot {
    pub corpses: HashMap<u64, WorldCorpse>,
    pub drops: HashMap<u64, AmmoDrop>,
    pub blaster_drops: HashMap<u64, BlasterDrop>,
    /// Seconds since last ammo claim send per drop (throttle while overlapping).
    claim_age_s: HashMap<u64, f32>,
}

impl WorldLoot {
    pub fn clear(&mut self) {
        self.corpses.clear();
        self.drops.clear();
        self.blaster_drops.clear();
        self.claim_age_s.clear();
    }

    pub fn note_corpse_spawn(&mut self, c: &NetCorpseSpawn) {
        self.corpses.insert(
            c.corpse_id,
            WorldCorpse {
                character: c.character,
                position: Vec3::new(c.position.x, c.position.y, c.position.z),
                facing: c.facing,
                die_age_s: 0.0,
            },
        );
    }

    pub fn note_corpse_end(&mut self, corpse_id: u64) {
        self.corpses.remove(&corpse_id);
    }

    pub fn note_drop_spawn(&mut self, d: &NetAmmoDropSpawn, kind: AmmoKind) {
        self.drops.insert(
            d.drop_id,
            AmmoDrop::new(
                d.drop_id,
                Vec3::new(d.position.x, d.position.y, d.position.z),
                kind,
                d.rounds,
            ),
        );
        self.claim_age_s.remove(&d.drop_id);
    }

    pub fn note_drop_end(&mut self, drop_id: u64) {
        self.drops.remove(&drop_id);
        self.claim_age_s.remove(&drop_id);
    }

    pub fn note_blaster_drop_spawn(&mut self, d: &NetBlasterDropSpawn) {
        self.blaster_drops.insert(
            d.drop_id,
            BlasterDrop::new(
                d.drop_id,
                Vec3::new(d.position.x, d.position.y, d.position.z),
                d.letter,
                d.mag,
                d.chamber,
            ),
        );
    }

    pub fn note_blaster_drop_end(&mut self, drop_id: u64) {
        self.blaster_drops.remove(&drop_id);
    }

    pub fn apply_grant_shrink(&mut self, drop_id: u64, rounds: u16) {
        let empty = if let Some(d) = self.drops.get_mut(&drop_id) {
            d.take_rounds(rounds);
            d.rounds == 0
        } else {
            false
        };
        if empty {
            self.note_drop_end(drop_id);
        }
    }

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        for c in self.corpses.values_mut() {
            c.die_age_s += dt;
        }
        for age in self.claim_age_s.values_mut() {
            *age += dt;
        }
        let expired: Vec<u64> = self
            .blaster_drops
            .values_mut()
            .map(|d| {
                d.tick(dt);
                d
            })
            .filter(|d| d.expired())
            .map(|d| d.id)
            .collect();
        for id in expired {
            self.note_blaster_drop_end(id);
        }
        // Solo ammo drops age locally too.
        let ammo_expired: Vec<u64> = self
            .drops
            .values_mut()
            .map(|d| {
                d.tick(dt);
                d
            })
            .filter(|d| d.expired())
            .map(|d| d.id)
            .collect();
        for id in ammo_expired {
            self.note_drop_end(id);
        }
    }

    /// Solo: spawn a local ammo drop with the given id.
    pub fn spawn_local_drop(&mut self, id: u64, position: Vec3, kind: AmmoKind, rounds: u16) {
        self.drops
            .insert(id, AmmoDrop::new(id, position, kind, rounds));
        self.claim_age_s.remove(&id);
    }

    /// Solo: spawn a local blaster drop.
    pub fn spawn_local_blaster_drop(
        &mut self,
        id: u64,
        position: Vec3,
        letter: u8,
        mag: u16,
        chamber: u16,
    ) {
        self.blaster_drops
            .insert(id, BlasterDrop::new(id, position, letter, mag, chamber));
    }

    /// Drops the living player overlaps with reserve room and may claim now.
    /// Returns `(drop_id, room)` for each claimable drop.
    pub fn overlapping_claimable(&mut self, state: &SelfState) -> Vec<(u64, u16)> {
        if !state.alive {
            return Vec::new();
        }
        // Forget throttle once out of radius.
        self.claim_age_s.retain(|id, _| {
            self.drops
                .get(id)
                .is_some_and(|d| d.rounds > 0 && in_take_radius(d.position, state.position))
        });
        let mut out = Vec::new();
        for d in self.drops.values() {
            if d.rounds == 0 {
                continue;
            }
            let room = state.reserve_room(d.kind);
            if room == 0 {
                continue;
            }
            if !in_take_radius(d.position, state.position) {
                continue;
            }
            let age = self
                .claim_age_s
                .get(&d.id)
                .copied()
                .unwrap_or(CLAIM_RETRY_S);
            if age < CLAIM_RETRY_S {
                continue;
            }
            out.push((d.id, room));
        }
        out
    }

    /// Blaster drop under the living player that they are looking at (F pickup, 067).
    pub fn overlapping_blaster(&self, state: &SelfState) -> Option<u64> {
        if !state.alive {
            return None;
        }
        let eye = loot_look_origin(state.position);
        let look = look_forward_dir(state.look_yaw(), state.look_pitch());
        self.blaster_drops.values().find_map(|d| {
            if !in_take_radius(d.position, state.position) {
                return None;
            }
            if !looking_at_blaster(eye, look, d.position) {
                return None;
            }
            Some(d.id)
        })
    }

    /// Dev HUD: drop count and nearest distance (m), if any.
    pub fn hud_near(&self, pos: Vec3) -> Option<(usize, f32, u16)> {
        if self.drops.is_empty() {
            return None;
        }
        let mut best: Option<(f32, u16)> = None;
        for d in self.drops.values() {
            let dist = d.position.distance(pos);
            if best.map(|(b, _)| dist < b).unwrap_or(true) {
                best = Some((dist, d.rounds));
            }
        }
        best.map(|(dist, rounds)| (self.drops.len(), dist, rounds))
    }

    pub fn mark_claimed(&mut self, drop_id: u64) {
        self.claim_age_s.insert(drop_id, 0.0);
    }

    /// Solo grant on overlap: returns (kind, rounds) granted to `state`.
    pub fn try_solo_take(&mut self, state: &mut SelfState) -> Option<(AmmoKind, u16)> {
        if !state.alive {
            return None;
        }
        let hit = self.drops.values().find_map(|d| {
            if d.rounds == 0 {
                return None;
            }
            let room = state.reserve_room(d.kind);
            if room == 0 || !in_take_radius(d.position, state.position) {
                return None;
            }
            Some((d.id, d.kind, d.take_amount(room)))
        })?;
        let (id, kind, want) = hit;
        let got = {
            let d = self.drops.get_mut(&id)?;
            d.take_rounds(want)
        };
        if got == 0 {
            return None;
        }
        let added = state.grant_reserve(kind, got);
        if self.drops.get(&id).is_some_and(|d| d.rounds == 0) {
            self.note_drop_end(id);
        }
        if added == 0 {
            None
        } else {
            Some((kind, added))
        }
    }

    /// Solo F take: equip floor blaster; returns displaced letter+mag+chamber for a new floor drop.
    pub fn try_solo_blaster_take(
        &mut self,
        state: &mut SelfState,
        drop_id: u64,
    ) -> Option<Option<(u8, u16, u16)>> {
        if !state.alive {
            return None;
        }
        let drop = self.blaster_drops.get(&drop_id)?;
        if !in_take_radius(drop.position, state.position) {
            return None;
        }
        let eye = loot_look_origin(state.position);
        let look = look_forward_dir(state.look_yaw(), state.look_pitch());
        if !looking_at_blaster(eye, look, drop.position) {
            return None;
        }
        let letter = drop.letter;
        let mag = drop.mag;
        let chamber = drop.chamber;
        self.note_blaster_drop_end(drop_id);
        let displaced = state.grant_floor_blaster(letter, mag, chamber).ok()?;
        Some(displaced)
    }
}
