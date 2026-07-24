//! First-class ammo kinds (042).
//!
//! Ammo owns round facts (mass). Blasters own which kind they fire and launch speed.

use crate::WeaponClass;

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
}
