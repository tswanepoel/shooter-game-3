//! Health and impact damage (043).

use glam::Vec3;

use crate::ammo::ammo_def;
use crate::fire::ProjectileWorld;
use crate::{AmmoKind, SelfState};

pub const HEALTH_MAX: f32 = 100.0;
/// Quiet time after last damage before regen starts.
pub const HEALTH_REGEN_DELAY_S: f32 = 3.0;
/// Empty → full once regen is running.
pub const HEALTH_REGEN_FULL_S: f32 = 6.0;
/// Health points per (kg·m/s) of impact momentum.
pub const IMPACT_DAMAGE_PER_MOMENTUM: f32 = 3.5;
/// Kenney `die` clip length; hold last frame after.
pub const DIE_DURATION_S: f32 = 0.33;

pub fn impact_damage(ammo: AmmoKind, speed_m_s: f32) -> f32 {
    let mass = ammo_def(ammo).mass;
    let speed = speed_m_s.max(0.0);
    mass * speed * IMPACT_DAMAGE_PER_MOMENTUM
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerHealth {
    pub health: f32,
    pub regen_block_s: f32,
    pub alive: bool,
    pub die_age_s: f32,
}

impl Default for PlayerHealth {
    fn default() -> Self {
        Self::full()
    }
}

impl PlayerHealth {
    pub fn full() -> Self {
        Self {
            health: HEALTH_MAX,
            regen_block_s: 0.0,
            alive: true,
            die_age_s: 0.0,
        }
    }

    /// Returns damage applied (0 if already dead).
    pub fn apply_impact(&mut self, ammo: AmmoKind, speed_m_s: f32) -> f32 {
        if !self.alive {
            return 0.0;
        }
        let damage = impact_damage(ammo, speed_m_s);
        if damage <= 0.0 {
            return 0.0;
        }
        self.health = (self.health - damage).max(0.0);
        self.regen_block_s = HEALTH_REGEN_DELAY_S;
        if self.health <= 0.0 {
            self.health = 0.0;
            self.alive = false;
            self.die_age_s = 0.0;
        }
        damage
    }

    pub fn tick_regen(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        if !self.alive {
            self.die_age_s += dt;
            return;
        }
        if self.regen_block_s > 0.0 {
            self.regen_block_s = (self.regen_block_s - dt).max(0.0);
            return;
        }
        if self.health < HEALTH_MAX && HEALTH_REGEN_FULL_S > 1e-6 {
            let rate = HEALTH_MAX / HEALTH_REGEN_FULL_S;
            self.health = (self.health + rate * dt).min(HEALTH_MAX);
        }
    }

    pub fn write_to_self(&self, state: &mut SelfState) {
        state.health = self.health;
        state.regen_block_s = self.regen_block_s;
        state.die_age_s = self.die_age_s;
        if self.alive {
            state.alive = true;
        } else if state.alive {
            state.alive = false;
            state.health = 0.0;
            state.die_age_s = self.die_age_s;
            state.sprint_latched = false;
            state.clear_emote();
            state.wish_forward = 0.0;
            state.wish_strafe = 0.0;
            if !state.locomotion.is_air() {
                state.locomotion = crate::LocomotionMode::Stand;
                state.walk_phase = 0.0;
            }
        } else {
            state.alive = false;
        }
    }

    pub fn read_from_self(state: &SelfState) -> Self {
        Self {
            health: state.health,
            regen_block_s: state.regen_block_s,
            alive: state.alive,
            die_age_s: state.die_age_s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImpactHit {
    pub target_id: u32,
    pub projectile_id: u64,
    pub firer_id: u32,
    pub ammo: AmmoKind,
    pub speed: f32,
    pub position: Vec3,
}

impl ProjectileWorld {
    /// Step motion; for own projectiles call `hit_test(from, to)` → `(target_id, contact)`.
    /// Does not apply health. Skips `owner != firer_id` (peer VFX).
    pub fn tick_hits_with<F>(&mut self, dt: f32, firer_id: u32, mut hit_test: F) -> Vec<ImpactHit>
    where
        F: FnMut(Vec3, Vec3) -> Option<(u32, Vec3)>,
    {
        let dt = dt.max(0.0);
        let mut hits = Vec::new();
        let mut spent = Vec::new();

        for (idx, p) in self.projectiles.iter_mut().enumerate() {
            let from = p.position;
            p.velocity += crate::PROJECTILE_GRAVITY * dt;
            let step = p.velocity * dt;
            let to = from + step;
            let speed = p.velocity.length();

            let mut hit_this = false;
            if p.owner == firer_id {
                if let Some((target_id, contact)) = hit_test(from, to) {
                    if target_id != firer_id {
                        hits.push(ImpactHit {
                            target_id,
                            projectile_id: p.id,
                            firer_id,
                            ammo: p.ammo,
                            speed,
                            position: contact,
                        });
                        hit_this = true;
                    }
                }
            }

            if hit_this {
                spent.push(idx);
            } else {
                p.position = to;
                p.traveled += step.length();
            }
        }

        for idx in spent.into_iter().rev() {
            self.projectiles.swap_remove(idx);
        }
        self.projectiles.retain(|p| p.traveled < p.max_range);
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ammo_def, AmmoKind, ProjectileWorld};

    fn foam_shot(pos: Vec3, vel: Vec3, owner: u32, ammo: AmmoKind) -> crate::Projectile {
        crate::Projectile {
            id: 1,
            owner,
            weapon: b'b',
            ammo,
            origin: pos,
            position: pos,
            velocity: vel,
            traveled: 0.0,
            max_range: 500.0,
            muzzle_index: 0,
        }
    }

    /// Stand-in for client mesh collide: hit target 2 if segment crosses z ∈ [4, 6] at y≈0.9.
    fn mock_mesh_hit(from: Vec3, to: Vec3) -> Option<(u32, Vec3)> {
        let dz = to.z - from.z;
        if dz.abs() < 1e-8 {
            return None;
        }
        let t = (5.0 - from.z) / dz;
        if !(0.0..=1.0).contains(&t) {
            return None;
        }
        let p = from + (to - from) * t;
        if (p.y - 0.9).abs() > 0.5 || p.x.abs() > 0.5 {
            return None;
        }
        Some((2, p))
    }

    #[test]
    fn impact_monotonic_in_speed_and_mass() {
        let slow = impact_damage(AmmoKind::LightFoam, 100.0);
        let fast = impact_damage(AmmoKind::LightFoam, 400.0);
        assert!(fast > slow);
        assert!(slow > 0.0);

        let light = impact_damage(AmmoKind::LightFoam, 400.0);
        let thick = impact_damage(AmmoKind::ThickFoam, 400.0);
        let grenade = impact_damage(AmmoKind::Grenade, 400.0);
        assert!(thick > light);
        assert!(grenade > thick);
        let m_l = ammo_def(AmmoKind::LightFoam).mass;
        let m_t = ammo_def(AmmoKind::ThickFoam).mass;
        assert!((thick / light - m_t / m_l).abs() < 1e-4);
    }

    #[test]
    fn firer_hit_claim_no_auto_health() {
        let mut world = ProjectileWorld::new();
        world.spawn(foam_shot(
            Vec3::new(0.0, 0.9, 0.0),
            Vec3::new(0.0, 0.0, 100.0),
            1,
            AmmoKind::LightFoam,
        ));
        let hits = world.tick_hits_with(0.1, 1, mock_mesh_hit);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].target_id, 2);
        assert_eq!(hits[0].firer_id, 1);
        assert_eq!(hits[0].ammo, AmmoKind::LightFoam);
        assert!(hits[0].speed > 0.0);
        assert!(world.projectiles.is_empty(), "spent on hit");
    }

    #[test]
    fn peer_vfx_projectile_does_not_claim_hit() {
        let mut world = ProjectileWorld::new();
        world.spawn(foam_shot(
            Vec3::new(0.0, 0.9, 0.0),
            Vec3::new(0.0, 0.0, 100.0),
            9,
            AmmoKind::LightFoam,
        ));
        let hits = world.tick_hits_with(0.1, 1, mock_mesh_hit);
        assert!(hits.is_empty());
        assert_eq!(world.projectiles.len(), 1);
    }

    #[test]
    fn firer_cannot_claim_self_as_target() {
        let mut world = ProjectileWorld::new();
        world.spawn(foam_shot(
            Vec3::new(0.0, 0.9, 0.0),
            Vec3::new(0.0, 0.0, 100.0),
            2,
            AmmoKind::LightFoam,
        ));
        // hit_test returns firer id 2 → rejected.
        let hits = world.tick_hits_with(0.1, 2, |from, to| {
            mock_mesh_hit(from, to).map(|(_, p)| (2, p))
        });
        assert!(hits.is_empty());
        assert_eq!(world.projectiles.len(), 1);
    }

    #[test]
    fn miss_callback_keeps_projectile() {
        let mut world = ProjectileWorld::new();
        world.spawn(foam_shot(
            Vec3::new(0.0, 0.9, 0.0),
            Vec3::new(0.0, 0.0, 100.0),
            1,
            AmmoKind::LightFoam,
        ));
        let hits = world.tick_hits_with(0.1, 1, |_from, _to| None);
        assert!(hits.is_empty());
        assert_eq!(world.projectiles.len(), 1);
    }

    #[test]
    fn apply_impact_drops_health_once() {
        let mut h = PlayerHealth::full();
        let d0 = h.apply_impact(AmmoKind::LightFoam, 400.0);
        assert!(d0 > 0.0);
        assert!(h.health < HEALTH_MAX);
        let mid = h.health;
        let d1 = h.apply_impact(AmmoKind::LightFoam, 400.0);
        assert!(d1 > 0.0);
        assert!(h.health < mid);
    }

    #[test]
    fn slower_impact_less_damage_via_translate() {
        let mut fast = PlayerHealth::full();
        let mut slow = PlayerHealth::full();
        let df = fast.apply_impact(AmmoKind::LightFoam, 400.0);
        let ds = slow.apply_impact(AmmoKind::LightFoam, 100.0);
        assert!(df > ds);
    }

    #[test]
    fn heavier_ammo_hurts_more_at_same_speed() {
        let speed = 200.0;
        assert!(
            impact_damage(AmmoKind::ThickFoam, speed) > impact_damage(AmmoKind::LightFoam, speed)
        );
    }

    #[test]
    fn regen_after_delay_and_reset_on_hit() {
        let mut h = PlayerHealth::full();
        h.apply_impact(AmmoKind::LightFoam, 400.0);
        let mid = h.health;
        h.tick_regen(HEALTH_REGEN_DELAY_S * 0.5);
        assert!((h.health - mid).abs() < 1e-4);
        h.tick_regen(HEALTH_REGEN_DELAY_S);
        h.tick_regen(1.0);
        assert!(h.health > mid);
        let before = h.health;
        h.apply_impact(AmmoKind::LightFoam, 100.0);
        assert!(h.health < before);
        assert!((h.regen_block_s - HEALTH_REGEN_DELAY_S).abs() < 1e-4);
    }

    #[test]
    fn zero_health_dead_no_respawn() {
        let mut h = PlayerHealth::full();
        while h.alive {
            h.apply_impact(AmmoKind::Grenade, 200.0);
        }
        assert!(!h.alive);
        assert_eq!(h.health, 0.0);
        assert_eq!(h.die_age_s, 0.0);
        h.tick_regen(0.1);
        assert!((h.die_age_s - 0.1).abs() < 1e-5);
        h.tick_regen(100.0);
        assert!(!h.alive);
        assert!(h.die_age_s > DIE_DURATION_S);
        assert_eq!(h.apply_impact(AmmoKind::LightFoam, 400.0), 0.0);
    }

    #[test]
    fn write_to_self_kills_living_actions() {
        let mut s = SelfState::default_loadout();
        let mut h = PlayerHealth::full();
        h.apply_impact(AmmoKind::Grenade, 500.0);
        while h.alive {
            h.apply_impact(AmmoKind::Grenade, 500.0);
        }
        h.write_to_self(&mut s);
        assert!(!s.alive);
        let pos = s.position;
        s.apply_move(0.1, 1.0, 0.0, true);
        assert_eq!(s.position, pos);
    }
}
