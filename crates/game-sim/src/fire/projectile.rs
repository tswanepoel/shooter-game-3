//! Projectiles in flight and one accepted discharge.

use glam::Vec3;

use crate::weapons::PROJECTILE_GRAVITY;
use crate::AmmoKind;

/// One projectile in flight (anemic bag; motion rules live on [`ProjectileWorld`]).
///
/// Carries ammo identity; mass is looked up via [`crate::ammo_def`], not stored here.
/// Launch speed is set from the blaster's muzzle velocity at spawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Projectile {
    pub id: u64,
    /// Shooter id (0 in solo when not networked).
    pub owner: u32,
    pub weapon: u8,
    pub ammo: AmmoKind,
    pub origin: Vec3,
    pub position: Vec3,
    pub velocity: Vec3,
    /// Path length from origin (m).
    pub traveled: f32,
    pub max_range: f32,
    /// Flash muzzle index when present has kit muzzles.
    pub muzzle_index: u8,
}

/// World set of live projectiles (self-claimed + accepted peer spawns).
#[derive(Debug, Clone, Default)]
pub struct ProjectileWorld {
    pub projectiles: Vec<Projectile>,
}

impl ProjectileWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self, p: Projectile) {
        self.projectiles.push(p);
    }

    pub fn spawn_many(&mut self, iter: impl IntoIterator<Item = Projectile>) {
        self.projectiles.extend(iter);
    }

    /// Gravity step + despawn when path length reaches max range.
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        for p in &mut self.projectiles {
            p.velocity += PROJECTILE_GRAVITY * dt;
            let step = p.velocity * dt;
            p.position += step;
            p.traveled += step.length();
        }
        self.projectiles.retain(|p| p.traveled < p.max_range);
    }

    pub fn clear(&mut self) {
        self.projectiles.clear();
    }
}

/// One accepted discharge: projectiles + present cues.
#[derive(Debug, Clone)]
pub struct Discharge {
    pub weapon: u8,
    pub projectiles: Vec<Projectile>,
    /// Muzzle indices that fired (unique, for flash).
    pub fired_muzzles: Vec<u8>,
}
