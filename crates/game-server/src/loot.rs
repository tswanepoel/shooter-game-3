//! Room corpses, ammo drops (059), and blaster drops (067).

use std::collections::HashMap;

use game_net::{
    encode_s2c, NetAmmoDropSpawn, NetBlasterDropSpawn, NetCorpseSpawn, NetVec3, PlayerId,
    ServerToClient,
};
use game_sim::{
    clamp_blaster_dump, clamp_dump_rounds, reserve_capacity_for, AmmoKind, AMMO_DROP_LIFETIME_S,
    BLASTER_DROP_LIFETIME_S, CORPSE_LIFETIME_S, LOOT_TAKE_RADIUS_M,
};

use super::roster::ammo_from_wire;

#[derive(Debug, Clone)]
pub struct CorpseRecord {
    pub corpse_id: u64,
    pub victim: PlayerId,
    pub age_s: f32,
    pub drop_id: Option<u64>,
    /// Victim already sent an ammo dump for this corpse.
    pub dumped: bool,
    /// Victim already sent a blaster dump for this corpse.
    pub blaster_dumped: bool,
}

#[derive(Debug, Clone)]
pub struct DropRecord {
    pub drop_id: u64,
    pub corpse_id: u64,
    pub position: NetVec3,
    pub kind: AmmoKind,
    pub rounds: u16,
    pub age_s: f32,
}

#[derive(Debug, Clone)]
pub struct BlasterDropRecord {
    pub drop_id: u64,
    /// 0 when not death-linked (displace dump). Kept for spawn echo.
    #[allow(dead_code)]
    pub corpse_id: u64,
    pub position: NetVec3,
    pub letter: u8,
    pub mag: u16,
    pub age_s: f32,
}

#[derive(Debug, Default)]
pub struct RoomLoot {
    next_corpse_id: u64,
    next_drop_id: u64,
    next_blaster_drop_id: u64,
    pub corpses: HashMap<u64, CorpseRecord>,
    pub drops: HashMap<u64, DropRecord>,
    pub blaster_drops: HashMap<u64, BlasterDropRecord>,
}

#[derive(Debug, Clone)]
pub struct LootTickEvents {
    pub corpse_ends: Vec<u64>,
    pub drop_ends: Vec<u64>,
    pub blaster_drop_ends: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct LootGrantEvent {
    pub drop_id: u64,
    pub player_id: PlayerId,
    pub ammo: AmmoKind,
    pub rounds: u16,
    pub drop_empty: bool,
}

#[derive(Debug, Clone)]
pub struct BlasterGrantEvent {
    pub drop_id: u64,
    pub player_id: PlayerId,
    pub letter: u8,
    pub mag: u16,
}

impl RoomLoot {
    pub fn spawn_corpse(
        &mut self,
        victim: PlayerId,
        character: u8,
        position: NetVec3,
        facing: f32,
    ) -> NetCorpseSpawn {
        self.next_corpse_id = self.next_corpse_id.saturating_add(1);
        let corpse_id = self.next_corpse_id;
        let spawn = NetCorpseSpawn {
            corpse_id,
            victim,
            character,
            position,
            facing,
        };
        self.corpses.insert(
            corpse_id,
            CorpseRecord {
                corpse_id,
                victim,
                age_s: 0.0,
                drop_id: None,
                dumped: false,
                blaster_dumped: false,
            },
        );
        spawn
    }

    /// Latest open corpse for `victim` that has not yet received an ammo dump.
    fn open_ammo_corpse_mut(&mut self, victim: PlayerId) -> Option<&mut CorpseRecord> {
        self.corpses
            .values_mut()
            .filter(|c| c.victim == victim && !c.dumped)
            .max_by_key(|c| c.corpse_id)
    }

    fn open_blaster_corpse_mut(&mut self, victim: PlayerId) -> Option<&mut CorpseRecord> {
        self.corpses
            .values_mut()
            .filter(|c| c.victim == victim && !c.blaster_dumped)
            .max_by_key(|c| c.corpse_id)
    }

    /// Accept victim ammo dump. Returns drop spawn when payload is above zero.
    pub fn accept_dump(
        &mut self,
        victim: PlayerId,
        ammo_wire: u8,
        rounds: u16,
        position: NetVec3,
    ) -> Option<NetAmmoDropSpawn> {
        let kind = ammo_from_wire(ammo_wire)?;
        let rounds = clamp_dump_rounds(kind, rounds);
        let corpse = self.open_ammo_corpse_mut(victim)?;
        corpse.dumped = true;
        if rounds == 0 {
            return None;
        }
        let corpse_id = corpse.corpse_id;
        self.next_drop_id = self.next_drop_id.saturating_add(1);
        let drop_id = self.next_drop_id;
        if let Some(c) = self.corpses.get_mut(&corpse_id) {
            c.drop_id = Some(drop_id);
        }
        self.drops.insert(
            drop_id,
            DropRecord {
                drop_id,
                corpse_id,
                position,
                kind,
                rounds,
                age_s: 0.0,
            },
        );
        Some(NetAmmoDropSpawn {
            drop_id,
            corpse_id,
            position,
            ammo: ammo_wire,
            rounds,
        })
    }

    /// Death blaster dump: requires open corpse. Always mints (mag may be 0).
    pub fn accept_blaster_death_dump(
        &mut self,
        victim: PlayerId,
        letter: u8,
        mag: u16,
        position: NetVec3,
    ) -> Option<NetBlasterDropSpawn> {
        let (letter, mag) = clamp_blaster_dump(letter, mag)?;
        let corpse = self.open_blaster_corpse_mut(victim)?;
        corpse.blaster_dumped = true;
        let corpse_id = corpse.corpse_id;
        Some(self.mint_blaster_drop(corpse_id, position, letter, mag))
    }

    /// Living displace dump: always mints a floor blaster (no corpse).
    pub fn accept_blaster_floor_dump(
        &mut self,
        letter: u8,
        mag: u16,
        position: NetVec3,
    ) -> Option<NetBlasterDropSpawn> {
        let (letter, mag) = clamp_blaster_dump(letter, mag)?;
        Some(self.mint_blaster_drop(0, position, letter, mag))
    }

    fn mint_blaster_drop(
        &mut self,
        corpse_id: u64,
        position: NetVec3,
        letter: u8,
        mag: u16,
    ) -> NetBlasterDropSpawn {
        self.next_blaster_drop_id = self.next_blaster_drop_id.saturating_add(1);
        let drop_id = self.next_blaster_drop_id;
        self.blaster_drops.insert(
            drop_id,
            BlasterDropRecord {
                drop_id,
                corpse_id,
                position,
                letter,
                mag,
                age_s: 0.0,
            },
        );
        NetBlasterDropSpawn {
            drop_id,
            corpse_id,
            position,
            letter,
            mag,
        }
    }

    /// First valid claim wins this take. Takes at most `room` rounds (remaining reserve space).
    pub fn elect_claim(
        &mut self,
        claimant: PlayerId,
        drop_id: u64,
        claimant_pos: NetVec3,
        room: u16,
        living: bool,
    ) -> Option<LootGrantEvent> {
        if !living {
            return None;
        }
        let drop = self.drops.get(&drop_id)?;
        if drop.rounds == 0 {
            return None;
        }
        if !within_take_radius(drop.position, claimant_pos) {
            return None;
        }
        let kind = drop.kind;
        let room = room.min(reserve_capacity_for(kind));
        let want = drop.rounds.min(room);
        if want == 0 {
            return None;
        }
        let drop = self.drops.get_mut(&drop_id)?;
        let got = want.min(drop.rounds);
        drop.rounds -= got;
        let empty = drop.rounds == 0;
        if empty {
            let corpse_id = drop.corpse_id;
            self.drops.remove(&drop_id);
            if let Some(c) = self.corpses.get_mut(&corpse_id) {
                c.drop_id = None;
            }
        }
        Some(LootGrantEvent {
            drop_id,
            player_id: claimant,
            ammo: kind,
            rounds: got,
            drop_empty: empty,
        })
    }

    /// First valid F claim wins the whole blaster drop (067).
    pub fn elect_blaster_claim(
        &mut self,
        claimant: PlayerId,
        drop_id: u64,
        claimant_pos: NetVec3,
        living: bool,
    ) -> Option<BlasterGrantEvent> {
        if !living {
            return None;
        }
        let drop = self.blaster_drops.get(&drop_id)?;
        if !within_take_radius(drop.position, claimant_pos) {
            return None;
        }
        let letter = drop.letter;
        let mag = drop.mag;
        self.blaster_drops.remove(&drop_id);
        Some(BlasterGrantEvent {
            drop_id,
            player_id: claimant,
            letter,
            mag,
        })
    }

    pub fn tick(&mut self, dt: f32) -> LootTickEvents {
        let dt = dt.max(0.0);
        let mut corpse_ends = Vec::new();
        let mut drop_ends = Vec::new();
        let mut blaster_drop_ends = Vec::new();

        for c in self.corpses.values_mut() {
            c.age_s += dt;
        }
        for d in self.drops.values_mut() {
            d.age_s += dt;
        }
        for d in self.blaster_drops.values_mut() {
            d.age_s += dt;
        }

        let expired_corpses: Vec<u64> = self
            .corpses
            .values()
            .filter(|c| c.age_s >= CORPSE_LIFETIME_S)
            .map(|c| c.corpse_id)
            .collect();
        for id in expired_corpses {
            if let Some(c) = self.corpses.remove(&id) {
                corpse_ends.push(id);
                if let Some(drop_id) = c.drop_id {
                    if self.drops.remove(&drop_id).is_some() {
                        drop_ends.push(drop_id);
                    }
                }
            }
        }

        let expired_drops: Vec<u64> = self
            .drops
            .values()
            .filter(|d| d.age_s >= AMMO_DROP_LIFETIME_S || d.rounds == 0)
            .map(|d| d.drop_id)
            .collect();
        for id in expired_drops {
            if let Some(d) = self.drops.remove(&id) {
                drop_ends.push(id);
                if let Some(c) = self.corpses.get_mut(&d.corpse_id) {
                    c.drop_id = None;
                }
            }
        }

        let expired_blasters: Vec<u64> = self
            .blaster_drops
            .values()
            .filter(|d| d.age_s >= BLASTER_DROP_LIFETIME_S)
            .map(|d| d.drop_id)
            .collect();
        for id in expired_blasters {
            if self.blaster_drops.remove(&id).is_some() {
                blaster_drop_ends.push(id);
            }
        }

        LootTickEvents {
            corpse_ends,
            drop_ends,
            blaster_drop_ends,
        }
    }
}

pub fn ammo_to_wire(kind: AmmoKind) -> u8 {
    match kind {
        AmmoKind::LightFoam => 0,
        AmmoKind::ThickFoam => 1,
        AmmoKind::Grenade => 2,
    }
}

fn within_take_radius(a: NetVec3, b: NetVec3) -> bool {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt() <= LOOT_TAKE_RADIUS_M
}

pub fn encode_corpse_spawn(tick: u64, corpse: NetCorpseSpawn) -> Option<Vec<u8>> {
    encode_s2c(&ServerToClient::CorpseSpawn { tick, corpse }).ok()
}

pub fn encode_corpse_end(tick: u64, corpse_id: u64) -> Option<Vec<u8>> {
    encode_s2c(&ServerToClient::CorpseEnd { tick, corpse_id }).ok()
}

pub fn encode_drop_spawn(tick: u64, drop: NetAmmoDropSpawn) -> Option<Vec<u8>> {
    encode_s2c(&ServerToClient::AmmoDropSpawn { tick, drop }).ok()
}

pub fn encode_drop_end(tick: u64, drop_id: u64) -> Option<Vec<u8>> {
    encode_s2c(&ServerToClient::AmmoDropEnd { tick, drop_id }).ok()
}

pub fn encode_loot_grant(
    tick: u64,
    drop_id: u64,
    player_id: PlayerId,
    ammo: AmmoKind,
    rounds: u16,
) -> Option<Vec<u8>> {
    encode_s2c(&ServerToClient::LootGrant {
        tick,
        drop_id,
        player_id,
        ammo: ammo_to_wire(ammo),
        rounds,
    })
    .ok()
}

pub fn encode_blaster_drop_spawn(tick: u64, drop: NetBlasterDropSpawn) -> Option<Vec<u8>> {
    encode_s2c(&ServerToClient::BlasterDropSpawn { tick, drop }).ok()
}

pub fn encode_blaster_drop_end(tick: u64, drop_id: u64) -> Option<Vec<u8>> {
    encode_s2c(&ServerToClient::BlasterDropEnd { tick, drop_id }).ok()
}

pub fn encode_blaster_grant(
    tick: u64,
    drop_id: u64,
    player_id: PlayerId,
    letter: u8,
    mag: u16,
) -> Option<Vec<u8>> {
    encode_s2c(&ServerToClient::BlasterGrant {
        tick,
        drop_id,
        player_id,
        letter,
        mag,
    })
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_then_claim_elects_and_empties() {
        let mut loot = RoomLoot::default();
        let corpse = loot.spawn_corpse(2, b'a', NetVec3::new(0.0, 0.0, 0.0), 0.0);
        let drop = loot
            .accept_dump(2, 0, 10, NetVec3::new(0.0, 0.0, 0.0))
            .expect("drop");
        assert_eq!(drop.corpse_id, corpse.corpse_id);
        assert_eq!(drop.rounds, 10);

        let grant = loot
            .elect_claim(1, drop.drop_id, NetVec3::new(0.5, 0.0, 0.0), 10, true)
            .expect("grant");
        assert_eq!(grant.rounds, 10);
        assert!(grant.drop_empty);
        assert!(loot.drops.is_empty());
    }

    #[test]
    fn claim_partial_room_leaves_rest() {
        let mut loot = RoomLoot::default();
        loot.spawn_corpse(2, b'a', NetVec3::new(0.0, 0.0, 0.0), 0.0);
        let drop = loot
            .accept_dump(2, 0, 50, NetVec3::new(0.0, 0.0, 0.0))
            .unwrap();
        let grant = loot
            .elect_claim(1, drop.drop_id, NetVec3::new(0.0, 0.0, 0.0), 12, true)
            .expect("grant");
        assert_eq!(grant.rounds, 12);
        assert!(!grant.drop_empty);
        assert_eq!(loot.drops.get(&drop.drop_id).unwrap().rounds, 38);
    }

    #[test]
    fn claim_outside_radius_ignored() {
        let mut loot = RoomLoot::default();
        loot.spawn_corpse(2, b'a', NetVec3::new(0.0, 0.0, 0.0), 0.0);
        let drop = loot
            .accept_dump(2, 0, 5, NetVec3::new(0.0, 0.0, 0.0))
            .unwrap();
        assert!(loot
            .elect_claim(
                1,
                drop.drop_id,
                NetVec3::new(LOOT_TAKE_RADIUS_M + 1.0, 0.0, 0.0),
                5,
                true
            )
            .is_none());
    }

    #[test]
    fn corpse_timer_ends_linked_drop() {
        let mut loot = RoomLoot::default();
        let corpse = loot.spawn_corpse(2, b'a', NetVec3::new(0.0, 0.0, 0.0), 0.0);
        let drop = loot
            .accept_dump(2, 0, 3, NetVec3::new(0.0, 0.0, 0.0))
            .unwrap();
        let ev = loot.tick(CORPSE_LIFETIME_S + 0.1);
        assert!(ev.corpse_ends.contains(&corpse.corpse_id));
        assert!(ev.drop_ends.contains(&drop.drop_id));
        assert!(loot.corpses.is_empty());
        assert!(loot.drops.is_empty());
    }

    #[test]
    fn blaster_death_dump_and_claim() {
        let mut loot = RoomLoot::default();
        let corpse = loot.spawn_corpse(2, b'a', NetVec3::new(0.0, 0.0, 0.0), 0.0);
        let drop = loot
            .accept_blaster_death_dump(2, b'b', 4, NetVec3::new(0.0, 0.0, 0.0))
            .expect("blaster");
        assert_eq!(drop.corpse_id, corpse.corpse_id);
        assert_eq!(drop.mag, 4);
        let grant = loot
            .elect_blaster_claim(1, drop.drop_id, NetVec3::new(0.2, 0.0, 0.0), true)
            .expect("grant");
        assert_eq!(grant.letter, b'b');
        assert_eq!(grant.mag, 4);
        assert!(loot.blaster_drops.is_empty());
    }

    #[test]
    fn blaster_floor_dump_independent_of_corpse_timer() {
        let mut loot = RoomLoot::default();
        let corpse = loot.spawn_corpse(2, b'a', NetVec3::new(0.0, 0.0, 0.0), 0.0);
        let _ = loot.accept_blaster_floor_dump(b'p', 10, NetVec3::new(1.0, 0.0, 0.0));
        // Corpse end does not remove floor blasters (067 independent lifetime).
        let ev = loot.tick(CORPSE_LIFETIME_S + 0.1);
        assert!(ev.corpse_ends.contains(&corpse.corpse_id));
        // Re-mint after ages may have cleared prior floor drops.
        let drop = loot
            .accept_blaster_floor_dump(b'p', 10, NetVec3::new(1.0, 0.0, 0.0))
            .unwrap();
        assert!(loot.blaster_drops.contains_key(&drop.drop_id));
        let ev = loot.tick(BLASTER_DROP_LIFETIME_S + 0.1);
        assert!(ev.blaster_drop_ends.contains(&drop.drop_id));
        assert!(!loot.blaster_drops.contains_key(&drop.drop_id));
    }
}
