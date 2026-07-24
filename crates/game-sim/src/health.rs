//! Health and impact damage (043 / 046).

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HitBodyPart {
    Head,
    Torso,
    ArmLeft,
    ArmRight,
    LegLeft,
    LegRight,
}

impl HitBodyPart {
    pub fn scale(self) -> f32 {
        match self {
            Self::Head => 2.0,
            Self::Torso => 1.0,
            Self::ArmLeft | Self::ArmRight => 0.85,
            Self::LegLeft | Self::LegRight => 0.75,
        }
    }

    pub fn kit_name(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Torso => "torso",
            Self::ArmLeft => "arm-left",
            Self::ArmRight => "arm-right",
            Self::LegLeft => "leg-left",
            Self::LegRight => "leg-right",
        }
    }

    pub fn from_kit_name(name: &str) -> Option<Self> {
        match name {
            "head" => Some(Self::Head),
            "torso" => Some(Self::Torso),
            "arm-left" => Some(Self::ArmLeft),
            "arm-right" => Some(Self::ArmRight),
            "leg-left" => Some(Self::LegLeft),
            "leg-right" => Some(Self::LegRight),
            _ => None,
        }
    }

    pub fn to_wire(self) -> u8 {
        match self {
            Self::Head => 0,
            Self::Torso => 1,
            Self::ArmLeft => 2,
            Self::ArmRight => 3,
            Self::LegLeft => 4,
            Self::LegRight => 5,
        }
    }

    pub fn from_wire(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Head),
            1 => Some(Self::Torso),
            2 => Some(Self::ArmLeft),
            3 => Some(Self::ArmRight),
            4 => Some(Self::LegLeft),
            5 => Some(Self::LegRight),
            _ => None,
        }
    }
}

pub fn impact_damage(ammo: AmmoKind, speed_m_s: f32, part: HitBodyPart) -> f32 {
    let mass = ammo_def(ammo).mass;
    let speed = speed_m_s.max(0.0);
    mass * speed * IMPACT_DAMAGE_PER_MOMENTUM * part.scale()
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
    pub fn apply_impact(&mut self, ammo: AmmoKind, speed_m_s: f32, part: HitBodyPart) -> f32 {
        if !self.alive {
            return 0.0;
        }
        let damage = impact_damage(ammo, speed_m_s, part);
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
    pub part: HitBodyPart,
}

impl ProjectileWorld {
    /// Step own projectiles; `hit_test(from, to)` → `(target_id, contact, part)`.
    /// No health apply. Peer VFX projectiles (`owner != firer_id`) are skipped.
    pub fn tick_hits_with<F>(&mut self, dt: f32, firer_id: u32, mut hit_test: F) -> Vec<ImpactHit>
    where
        F: FnMut(Vec3, Vec3) -> Option<(u32, Vec3, HitBodyPart)>,
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
                if let Some((target_id, contact, part)) = hit_test(from, to) {
                    if target_id != firer_id {
                        hits.push(ImpactHit {
                            target_id,
                            projectile_id: p.id,
                            firer_id,
                            ammo: p.ammo,
                            speed,
                            position: contact,
                            part,
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
    fn mock_mesh_hit(from: Vec3, to: Vec3) -> Option<(u32, Vec3, HitBodyPart)> {
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
        Some((2, p, HitBodyPart::Torso))
    }

    #[test]
    fn impact_monotonic_in_speed_and_mass() {
        let slow = impact_damage(AmmoKind::LightFoam, 100.0, HitBodyPart::Torso);
        let fast = impact_damage(AmmoKind::LightFoam, 400.0, HitBodyPart::Torso);
        assert!(fast > slow);
        assert!(slow > 0.0);

        let light = impact_damage(AmmoKind::LightFoam, 400.0, HitBodyPart::Torso);
        let thick = impact_damage(AmmoKind::ThickFoam, 400.0, HitBodyPart::Torso);
        let grenade = impact_damage(AmmoKind::Grenade, 400.0, HitBodyPart::Torso);
        assert!(thick > light);
        assert!(grenade > thick);
        let m_l = ammo_def(AmmoKind::LightFoam).mass;
        let m_t = ammo_def(AmmoKind::ThickFoam).mass;
        assert!((thick / light - m_t / m_l).abs() < 1e-4);
    }

    #[test]
    fn part_scale_orders_damage() {
        let ammo = AmmoKind::LightFoam;
        let speed = 400.0;
        let head = impact_damage(ammo, speed, HitBodyPart::Head);
        let torso = impact_damage(ammo, speed, HitBodyPart::Torso);
        let arm = impact_damage(ammo, speed, HitBodyPart::ArmLeft);
        let leg = impact_damage(ammo, speed, HitBodyPart::LegRight);
        assert!((head / torso - 2.0).abs() < 1e-4);
        assert!((arm / torso - 0.85).abs() < 1e-4);
        assert!((leg / torso - 0.75).abs() < 1e-4);
        assert!(head > torso && torso > arm && arm > leg);
    }

    #[test]
    fn kit_name_and_wire_roundtrip() {
        for p in [
            HitBodyPart::Head,
            HitBodyPart::Torso,
            HitBodyPart::ArmLeft,
            HitBodyPart::ArmRight,
            HitBodyPart::LegLeft,
            HitBodyPart::LegRight,
        ] {
            assert_eq!(HitBodyPart::from_kit_name(p.kit_name()), Some(p));
            assert_eq!(HitBodyPart::from_wire(p.to_wire()), Some(p));
        }
        assert_eq!(HitBodyPart::from_kit_name("root"), None);
        assert_eq!(HitBodyPart::from_wire(9), None);
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
        assert_eq!(hits[0].part, HitBodyPart::Torso);
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
            mock_mesh_hit(from, to).map(|(_, p, part)| (2, p, part))
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
        let d0 = h.apply_impact(AmmoKind::LightFoam, 400.0, HitBodyPart::Torso);
        assert!(d0 > 0.0);
        assert!(h.health < HEALTH_MAX);
        let mid = h.health;
        let d1 = h.apply_impact(AmmoKind::LightFoam, 400.0, HitBodyPart::Torso);
        assert!(d1 > 0.0);
        assert!(h.health < mid);
    }

    #[test]
    fn head_impact_hurts_more_than_leg() {
        let mut head = PlayerHealth::full();
        let mut leg = PlayerHealth::full();
        let dh = head.apply_impact(AmmoKind::LightFoam, 400.0, HitBodyPart::Head);
        let dl = leg.apply_impact(AmmoKind::LightFoam, 400.0, HitBodyPart::LegLeft);
        assert!(dh > dl);
        assert!(head.health < leg.health);
    }

    #[test]
    fn slower_impact_less_damage_via_translate() {
        let mut fast = PlayerHealth::full();
        let mut slow = PlayerHealth::full();
        let df = fast.apply_impact(AmmoKind::LightFoam, 400.0, HitBodyPart::Torso);
        let ds = slow.apply_impact(AmmoKind::LightFoam, 100.0, HitBodyPart::Torso);
        assert!(df > ds);
    }

    #[test]
    fn heavier_ammo_hurts_more_at_same_speed() {
        let speed = 200.0;
        assert!(
            impact_damage(AmmoKind::ThickFoam, speed, HitBodyPart::Torso)
                > impact_damage(AmmoKind::LightFoam, speed, HitBodyPart::Torso)
        );
    }

    #[test]
    fn regen_after_delay_and_reset_on_hit() {
        let mut h = PlayerHealth::full();
        h.apply_impact(AmmoKind::LightFoam, 400.0, HitBodyPart::Torso);
        let mid = h.health;
        h.tick_regen(HEALTH_REGEN_DELAY_S * 0.5);
        assert!((h.health - mid).abs() < 1e-4);
        h.tick_regen(HEALTH_REGEN_DELAY_S);
        h.tick_regen(1.0);
        assert!(h.health > mid);
        let before = h.health;
        h.apply_impact(AmmoKind::LightFoam, 100.0, HitBodyPart::Torso);
        assert!(h.health < before);
        assert!((h.regen_block_s - HEALTH_REGEN_DELAY_S).abs() < 1e-4);
    }

    #[test]
    fn zero_health_dead_no_respawn() {
        let mut h = PlayerHealth::full();
        while h.alive {
            h.apply_impact(AmmoKind::Grenade, 200.0, HitBodyPart::Torso);
        }
        assert!(!h.alive);
        assert_eq!(h.health, 0.0);
        assert_eq!(h.die_age_s, 0.0);
        h.tick_regen(0.1);
        assert!((h.die_age_s - 0.1).abs() < 1e-5);
        h.tick_regen(100.0);
        assert!(!h.alive);
        assert!(h.die_age_s > DIE_DURATION_S);
        assert_eq!(
            h.apply_impact(AmmoKind::LightFoam, 400.0, HitBodyPart::Head),
            0.0
        );
    }

    #[test]
    fn write_to_self_kills_living_actions() {
        let mut s = SelfState::default_loadout();
        let mut h = PlayerHealth::full();
        h.apply_impact(AmmoKind::Grenade, 500.0, HitBodyPart::Torso);
        while h.alive {
            h.apply_impact(AmmoKind::Grenade, 500.0, HitBodyPart::Torso);
        }
        h.write_to_self(&mut s);
        assert!(!s.alive);
        let pos = s.position;
        s.apply_move(0.1, 1.0, 0.0, true);
        assert_eq!(s.position, pos);
    }
}
