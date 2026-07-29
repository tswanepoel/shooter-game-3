//! Fire, projectiles, hits, health, and loot for one frame.

use std::collections::HashMap;

use game_net::PlayerId;
use game_sim::{HitBodyPart, PlayerHealth, SelfState};

use crate::mp;
use crate::self_present::SelfPresentState;

use super::super::impact::apply_impact_in_present;
use super::super::ClientInner;

impl ClientInner {
    pub(super) fn tick_combat(&mut self, dt: f32, fire_held: bool) {
        self.self_state.tick_emote(dt);

        if fire_held && self.self_state.is_emoting() {
            self.self_state.clear_emote();
        }
        let owner = self.mp.player_id().unwrap_or(0);
        let look_origin = match &self.self_present {
            SelfPresentState::Ready(gpu) => gpu.view.look_origin,
            _ => self.self_state.position + glam::Vec3::new(0.0, 1.52, 0.27),
        };
        let muzzle_worlds = match &self.self_present {
            SelfPresentState::Ready(gpu) => gpu.fire_muzzle_worlds(&self.self_state),
            _ => Vec::new(),
        };
        let discharges = self.fire.tick(
            dt,
            &mut self.self_state,
            fire_held,
            owner,
            look_origin,
            &muzzle_worlds,
        );
        let mut claimed: Vec<game_sim::Projectile> = Vec::new();
        for d in &discharges {
            for p in &d.projectiles {
                claimed.push(p.clone());
                self.projectiles.spawn(p.clone());
            }
            let seed_pts = match &self.self_present {
                SelfPresentState::Ready(gpu) => {
                    gpu.flash_muzzle_worlds(&self.self_state, &d.fired_muzzles)
                }
                _ => muzzle_worlds.clone(),
            };
            self.renderer
                .fire_fx
                .note_self_discharge(&d.fired_muzzles, &seed_pts);
        }

        if !claimed.is_empty() {
            self.mp.claim_projectiles(&claimed);
        }

        // Accept peer projectiles (claim-and-relay).
        for batch in self.mp.take_peer_projectiles() {
            let mut muzzle_indices = Vec::new();
            let mut seed_pts = Vec::new();
            let mut weapon = b'p';
            for n in &batch.projectiles {
                if let Some(p) = mp::net_spawn_to_projectile(batch.id, n) {
                    weapon = p.weapon;
                    if !muzzle_indices.contains(&p.muzzle_index) {
                        muzzle_indices.push(p.muzzle_index);
                        seed_pts.push(p.origin);
                    }
                    self.projectiles.spawn(p);
                }
            }
            if !muzzle_indices.is_empty() {
                self.renderer.fire_fx.note_peer_projectiles(
                    batch.id,
                    weapon,
                    &muzzle_indices,
                    &seed_pts,
                );
            }
        }

        let local_id = self.mp.player_id();
        let firer_id = local_id.unwrap_or(0);
        if !self.mp.in_room() {
            self.health_by_id.clear();
        }

        let remote_samples: Vec<(PlayerId, game_net::DriveView)> = if self.mp.in_room() {
            self.mp
                .remotes()
                .samples()
                .map(|(id, s)| (id, s.drive.clone()))
                .collect()
        } else {
            Vec::new()
        };

        if let Some(id) = local_id {
            self.health_by_id
                .entry(id)
                .or_insert_with(|| PlayerHealth::read_from_self(&self.self_state));
        }
        for (id, _) in &remote_samples {
            let living = self.mp.peer_living(*id);
            self.health_by_id.entry(*id).or_insert_with(|| {
                if living {
                    PlayerHealth::full()
                } else {
                    // Unknown corpse (e.g. after local health clear): hold die end.
                    PlayerHealth {
                        health: 0.0,
                        regen_block_s: 0.0,
                        alive: false,
                        die_age_s: game_sim::DIE_DURATION_S,
                    }
                }
            });
        }
        if self.mp.in_room() {
            let mut keep: HashMap<PlayerId, ()> =
                remote_samples.iter().map(|(id, _)| (*id, ())).collect();
            if let Some(id) = local_id {
                keep.insert(id, ());
            }
            self.health_by_id.retain(|id, _| keep.contains_key(id));
            // Roster living is membership truth (053 respawn). Local health must
            // clear die pose when a peer re-enters; otherwise remotes stay at die
            // last frame while drive walks (corpse snake).
            for (id, _) in &remote_samples {
                let living = self.mp.peer_living(*id);
                let Some(h) = self.health_by_id.get_mut(id) else {
                    continue;
                };
                if living && !h.alive {
                    *h = PlayerHealth::full();
                } else if !living && h.alive {
                    h.health = 0.0;
                    h.regen_block_s = 0.0;
                    h.alive = false;
                    h.die_age_s = 0.0;
                }
            }
        }

        let remote_hit_states: Vec<(PlayerId, SelfState)> = remote_samples
            .iter()
            .filter_map(|(id, drive)| {
                if !self.mp.peer_living(*id) {
                    return None;
                }
                let h = self.health_by_id.get(id)?;
                if !h.alive {
                    return None;
                }
                let mut state = mp::drive_to_state(drive);
                state.alive = true;
                state.die_age_s = 0.0;
                Some((*id, state))
            })
            .collect();

        let hits = {
            let remote_present = &self.remote_present;
            let states = &remote_hit_states;
            let map_world = &self.map_world;
            self.projectiles
                .tick_hits_with(dt, firer_id, |origin, from, to| {
                    let mut best: Option<(f32, PlayerId, glam::Vec3, HitBodyPart)> = None;
                    for (id, state) in states {
                        let Some(hit) = remote_present.trace_segment(*id, state, from, to) else {
                            continue;
                        };
                        let Some(part) = HitBodyPart::from_kit_name(&hit.part) else {
                            continue;
                        };
                        // Block if any solid lies between the spawn origin and just
                        // short of the contact point. Pulling the endpoint back by a
                        // small margin prevents the box surface itself from occluding
                        // a shot at a target standing on or against that box.
                        let ray = hit.position - origin;
                        let ray_len = ray.length();
                        let check_end = if ray_len > 0.3 {
                            origin + ray * ((ray_len - 0.15) / ray_len)
                        } else {
                            hit.position
                        };
                        if map_world.segment_hits_solid(origin, check_end) {
                            continue;
                        }
                        if best.map(|(bt, _, _, _)| hit.t < bt).unwrap_or(true) {
                            best = Some((hit.t, *id, hit.position, part));
                        }
                    }
                    best.map(|(_, id, p, part)| (id, p, part))
                })
        };
        for h in &hits {
            let dmg = apply_impact_in_present(
                h.target_id,
                h.ammo,
                h.speed,
                h.part,
                local_id,
                &mut self.self_state,
                &mut self.health_by_id,
            );
            if dmg > 0.0 {
                self.fire.add_hit_impulse(&mut self.self_state, dmg);
            }
        }
        if !hits.is_empty() {
            self.mp.claim_hits(&hits);
            // One flash per claim batch; successive frames re-pulse (044).
            self.hit_marker.pulse();
        }

        self.hit_marker.tick(dt);

        for batch in self.mp.take_peer_hits() {
            let Some(ammo) = mp::ammo_kind_from_wire(batch.hit.ammo) else {
                continue;
            };
            let Some(part) = HitBodyPart::from_wire(batch.hit.part) else {
                continue;
            };
            let dmg = apply_impact_in_present(
                batch.hit.target,
                ammo,
                batch.hit.speed,
                part,
                local_id,
                &mut self.self_state,
                &mut self.health_by_id,
            );
            if dmg > 0.0 {
                self.fire.add_hit_impulse(&mut self.self_state, dmg);
            }
        }

        self.self_state.tick_health(dt);
        if let Some(id) = local_id {
            self.health_by_id
                .insert(id, PlayerHealth::read_from_self(&self.self_state));
        }
        for (id, h) in self.health_by_id.iter_mut() {
            if local_id == Some(*id) {
                continue;
            }
            h.tick_regen(dt);
        }

        // Death dump → victim AmmoDump / solo local drop (059).
        if self.was_alive && !self.self_state.alive {
            let pos = self.self_state.position;
            if let Some((kind, rounds)) = self.self_state.dump_death_ammo() {
                if self.mp.in_room() {
                    self.mp.claim_ammo_dump(kind, rounds, pos);
                } else if rounds > 0 {
                    let id = self.next_local_drop_id;
                    self.next_local_drop_id = self.next_local_drop_id.saturating_add(1);
                    self.world_loot.spawn_local_drop(id, pos, kind, rounds);
                }
            }
        }
        self.was_alive = self.self_state.alive;

        if self.mp.is_living() && !self.self_state.alive {
            self.mp.return_to_bench_after_death(&self.self_state);
        }

        // Ingest room loot announces / ends / grants.
        for c in self.mp.take_corpse_spawns() {
            self.world_loot.note_corpse_spawn(&c);
        }
        for id in self.mp.take_corpse_ends() {
            self.world_loot.note_corpse_end(id);
        }
        for d in self.mp.take_drop_spawns() {
            let Some(kind) = mp::ammo_kind_from_wire(d.ammo) else {
                continue;
            };
            self.world_loot.note_drop_spawn(&d, kind);
        }
        for id in self.mp.take_drop_ends() {
            self.world_loot.note_drop_end(id);
        }
        for g in self.mp.take_loot_grants() {
            self.world_loot.apply_grant_shrink(g.drop_id, g.rounds);
            if local_id == Some(g.player_id) {
                if let Some(kind) = mp::ammo_kind_from_wire(g.ammo) {
                    self.self_state.grant_reserve(kind, g.rounds);
                }
            }
        }

        self.world_loot.tick(dt);

        // Walk-over: MP claims; solo grants locally.
        if self.self_state.alive {
            if self.mp.is_living() {
                for (drop_id, room) in self.world_loot.overlapping_claimable(&self.self_state) {
                    self.mp.claim_loot(drop_id, self.self_state.position, room);
                    self.world_loot.mark_claimed(drop_id);
                }
            } else if !self.mp.in_room() {
                let _ = self.world_loot.try_solo_take(&mut self.self_state);
            }
        }

        self.renderer.fire_fx.tick(dt);

        self.mp.on_frame(dt, &self.self_state);
    }
}
