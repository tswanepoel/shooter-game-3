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

/// Rounds of each ammo kind carried outside any blaster (058).
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

/// Per-letter chamber / seat / mag / load / spare (081).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LetterAmmo {
    pub chamber: u16,
    /// Timed seat batch size: `0` seats instantly; `N` seats one round at a time (081).
    pub seat: u16,
    pub mag: u16,
    pub load: u16,
    pub spare: u16,
}

/// Look up letter `a`..=`r` ammo tune.
pub fn letter_ammo(letter: u8) -> Option<&'static LetterAmmo> {
    let i = (letter as usize).checked_sub(b'a' as usize)?;
    LETTER_AMMO.get(i)
}

pub fn spawn_spare_for_letter(letter: u8) -> u16 {
    letter_ammo(letter).map(|t| t.spare).unwrap_or(0)
}

pub fn spawn_mag_for_letter(letter: u8) -> u16 {
    letter_ammo(letter).map(|t| t.load.min(t.mag)).unwrap_or(0)
}

const LETTER_AMMO: [LetterAmmo; 18] = [
    LetterAmmo {
        chamber: 1,
        seat: 1,
        mag: 0,
        load: 0,
        spare: 2,
    }, // a
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 12,
        load: 8,
        spare: 30,
    }, // b
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 24,
        load: 20,
        spare: 60,
    }, // c
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 24,
        load: 16,
        spare: 48,
    }, // d
    LetterAmmo {
        chamber: 1,
        seat: 1,
        mag: 6,
        load: 4,
        spare: 12,
    }, // e
    LetterAmmo {
        chamber: 1,
        seat: 1,
        mag: 6,
        load: 4,
        spare: 12,
    }, // f
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 24,
        load: 20,
        spare: 60,
    }, // g
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 24,
        load: 20,
        spare: 60,
    }, // h
    LetterAmmo {
        chamber: 2,
        seat: 2,
        mag: 0,
        load: 0,
        spare: 30,
    }, // i
    LetterAmmo {
        chamber: 2,
        seat: 2,
        mag: 0,
        load: 0,
        spare: 24,
    }, // j
    LetterAmmo {
        chamber: 1,
        seat: 1,
        mag: 6,
        load: 4,
        spare: 18,
    }, // k
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 24,
        load: 20,
        spare: 60,
    }, // l
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 24,
        load: 20,
        spare: 60,
    }, // m
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 20,
        load: 16,
        spare: 48,
    }, // n
    LetterAmmo {
        chamber: 4,
        seat: 4,
        mag: 0,
        load: 0,
        spare: 24,
    }, // o
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 24,
        load: 20,
        spare: 60,
    }, // p
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 20,
        load: 16,
        spare: 48,
    }, // q
    LetterAmmo {
        chamber: 1,
        seat: 0,
        mag: 20,
        load: 16,
        spare: 48,
    }, // r
];

/// Max reserve rounds a player may carry per kind (059 / 081): ~2× largest spawn spare.
pub fn reserve_capacity_for(kind: AmmoKind) -> u16 {
    match kind {
        AmmoKind::LightFoam => 120,
        AmmoKind::ThickFoam => 24,
        AmmoKind::Grenade => 4,
    }
}

/// Upper bound for a death ammo dump of `kind` (reserve only; mag rides blaster drop, 067).
pub fn max_dump_rounds_for(kind: AmmoKind) -> u16 {
    reserve_capacity_for(kind)
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
