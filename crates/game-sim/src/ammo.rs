//! Ammo owns round facts (mass). Blasters own which kind they fire and launch speed.
//! Magazine capacity and spawn reserve drafts live here as tune tables (058).
//! Reserve capacity and death-dump / loot take drafts (059).

use crate::WeaponClass;

/// Corpse present lifetime (s). Long enough to loot (059).
pub const CORPSE_LIFETIME_S: f32 = 45.0;
/// Ammo drop lifetime (s). May end earlier on grant-empty or corpse end (059).
pub const AMMO_DROP_LIFETIME_S: f32 = 45.0;
/// Blaster drop lifetime (s). Short; ends earlier on grant (067).
pub const BLASTER_DROP_LIFETIME_S: f32 = 20.0;
/// Walk-over / F take radius (m) around a floor drop (059 / 067).
pub const LOOT_TAKE_RADIUS_M: f32 = 1.5;
/// Death blaster drop: right of corpse (right-handed hold), metres (067).
pub const DEATH_BLASTER_RIGHT_M: f32 = 0.55;
/// Death blaster drop: behind feet (die falls backwards), metres (067).
pub const DEATH_BLASTER_BACK_M: f32 = 0.4;
/// Swap / displace dump in front of feet, metres (067).
pub const SWAP_BLASTER_FORWARD_M: f32 = 0.85;
/// Min look·to_drop for F pickup (~25° half-angle) (067).
pub const BLASTER_LOOK_DOT_MIN: f32 = 0.9063;

/// What a round is. Shared across letters that fire the same kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AmmoKind {
    /// Light foam dart / pellet.
    LightFoam,
    /// Heavy foam slug (sniper).
    ThickFoam,
    /// Launcher round.
    Grenade,
}

/// Rounds of each ammo kind carried outside any magazine (058).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReserveAmmo {
    pub light_foam: u16,
    pub thick_foam: u16,
    pub grenade: u16,
}

impl ReserveAmmo {
    pub fn get(self, kind: AmmoKind) -> u16 {
        match kind {
            AmmoKind::LightFoam => self.light_foam,
            AmmoKind::ThickFoam => self.thick_foam,
            AmmoKind::Grenade => self.grenade,
        }
    }

    pub fn set(&mut self, kind: AmmoKind, n: u16) {
        match kind {
            AmmoKind::LightFoam => self.light_foam = n,
            AmmoKind::ThickFoam => self.thick_foam = n,
            AmmoKind::Grenade => self.grenade = n,
        }
    }

    /// Remove up to `n` rounds of `kind`; returns how many were removed.
    pub fn take(&mut self, kind: AmmoKind, n: u16) -> u16 {
        let have = self.get(kind);
        let spent = have.min(n);
        self.set(kind, have - spent);
        spent
    }

    /// Add rounds of `kind` (saturating).
    pub fn add(&mut self, kind: AmmoKind, n: u16) {
        let sum = self.get(kind).saturating_add(n);
        self.set(kind, sum);
    }

    /// How many more rounds of `kind` fit under reserve capacity (059).
    pub fn room(self, kind: AmmoKind) -> u16 {
        reserve_capacity_for(kind).saturating_sub(self.get(kind))
    }

    /// Add up to `n` rounds of `kind`, capped by reserve capacity. Returns how many added.
    pub fn add_capped(&mut self, kind: AmmoKind, n: u16) -> u16 {
        let fit = self.room(kind).min(n);
        self.add(kind, fit);
        fit
    }
}

/// Draft magazine capacity by weapon class (058).
pub fn mag_capacity_for_class(class: WeaponClass) -> u16 {
    match class {
        WeaponClass::Launcher => 1,
        WeaponClass::Pistol => 12,
        WeaponClass::Smg | WeaponClass::AssaultRifle | WeaponClass::Shotgun => 30,
        WeaponClass::SniperRifle => 5,
    }
}

/// Spawn reserve draft for a kind the loadout uses (058).
pub fn spawn_reserve_for(kind: AmmoKind) -> u16 {
    match kind {
        AmmoKind::LightFoam => 90,
        AmmoKind::ThickFoam => 20,
        AmmoKind::Grenade => 4,
    }
}

/// Max reserve rounds a player may carry per kind (059).
pub fn reserve_capacity_for(kind: AmmoKind) -> u16 {
    match kind {
        AmmoKind::LightFoam => 180,
        AmmoKind::ThickFoam => 40,
        AmmoKind::Grenade => 8,
    }
}

/// Upper bound for a death ammo dump of `kind` (reserve only; mag rides blaster drop, 067).
pub fn max_dump_rounds_for(kind: AmmoKind) -> u16 {
    reserve_capacity_for(kind)
}

/// Clamp magazine rounds for a floor blaster letter (067).
pub fn clamp_blaster_mag(letter: u8, mag: u16) -> Option<u16> {
    let def = crate::weapon_def(letter)?;
    Some(mag.min(def.mag_capacity()))
}

/// Round-only facts. Mass is the source of truth here — not on the blaster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmmoDef {
    pub kind: AmmoKind,
    /// Mass (kg).
    pub mass: f32,
}

/// Light foam (pistol / SMG / AR / shotgun pellet).
pub const MASS_LIGHT_FOAM_KG: f32 = 0.008;
/// Thick foam slug (sniper).
pub const MASS_THICK_FOAM_KG: f32 = 0.035;
/// Grenade (launcher).
pub const MASS_GRENADE_KG: f32 = 0.25;

const AMMO_TABLE: [AmmoDef; 3] = [
    AmmoDef {
        kind: AmmoKind::LightFoam,
        mass: MASS_LIGHT_FOAM_KG,
    },
    AmmoDef {
        kind: AmmoKind::ThickFoam,
        mass: MASS_THICK_FOAM_KG,
    },
    AmmoDef {
        kind: AmmoKind::Grenade,
        mass: MASS_GRENADE_KG,
    },
];

/// Lookup round facts by kind.
pub fn ammo_def(kind: AmmoKind) -> &'static AmmoDef {
    match kind {
        AmmoKind::LightFoam => &AMMO_TABLE[0],
        AmmoKind::ThickFoam => &AMMO_TABLE[1],
        AmmoKind::Grenade => &AMMO_TABLE[2],
    }
}

/// Blaster class → ammo kind. No per-letter overrides (042).
pub fn ammo_for_class(class: WeaponClass) -> AmmoKind {
    match class {
        WeaponClass::SniperRifle => AmmoKind::ThickFoam,
        WeaponClass::Launcher => AmmoKind::Grenade,
        WeaponClass::Pistol
        | WeaponClass::Smg
        | WeaponClass::AssaultRifle
        | WeaponClass::Shotgun => AmmoKind::LightFoam,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WeaponClass;

    #[test]
    fn mass_tiers_ordered() {
        assert!(ammo_def(AmmoKind::LightFoam).mass < ammo_def(AmmoKind::ThickFoam).mass);
        assert!(ammo_def(AmmoKind::ThickFoam).mass < ammo_def(AmmoKind::Grenade).mass);
        assert!(ammo_def(AmmoKind::LightFoam).mass > 0.0);
    }

    #[test]
    fn class_ammo_map() {
        assert_eq!(ammo_for_class(WeaponClass::Pistol), AmmoKind::LightFoam);
        assert_eq!(ammo_for_class(WeaponClass::Smg), AmmoKind::LightFoam);
        assert_eq!(
            ammo_for_class(WeaponClass::AssaultRifle),
            AmmoKind::LightFoam
        );
        assert_eq!(ammo_for_class(WeaponClass::Shotgun), AmmoKind::LightFoam);
        assert_eq!(
            ammo_for_class(WeaponClass::SniperRifle),
            AmmoKind::ThickFoam
        );
        assert_eq!(ammo_for_class(WeaponClass::Launcher), AmmoKind::Grenade);
    }

    #[test]
    fn mag_capacity_by_class() {
        assert_eq!(mag_capacity_for_class(WeaponClass::Launcher), 1);
        assert_eq!(mag_capacity_for_class(WeaponClass::Pistol), 12);
        assert_eq!(mag_capacity_for_class(WeaponClass::Smg), 30);
        assert_eq!(mag_capacity_for_class(WeaponClass::AssaultRifle), 30);
        assert_eq!(mag_capacity_for_class(WeaponClass::Shotgun), 30);
        assert_eq!(mag_capacity_for_class(WeaponClass::SniperRifle), 5);
    }

    #[test]
    fn reserve_take_and_add() {
        let mut r = ReserveAmmo::default();
        r.add(AmmoKind::LightFoam, 10);
        assert_eq!(r.take(AmmoKind::LightFoam, 4), 4);
        assert_eq!(r.get(AmmoKind::LightFoam), 6);
        assert_eq!(r.take(AmmoKind::LightFoam, 100), 6);
        assert_eq!(r.get(AmmoKind::LightFoam), 0);
    }

    #[test]
    fn reserve_room_and_capped_add() {
        let mut r = ReserveAmmo::default();
        r.set(
            AmmoKind::Grenade,
            reserve_capacity_for(AmmoKind::Grenade) - 1,
        );
        assert_eq!(r.room(AmmoKind::Grenade), 1);
        assert_eq!(r.add_capped(AmmoKind::Grenade, 5), 1);
        assert_eq!(r.room(AmmoKind::Grenade), 0);
        assert_eq!(r.add_capped(AmmoKind::Grenade, 1), 0);
    }
}
