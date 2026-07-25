//! Loadout and locomotion enums for SelfState.

/// Locomotion mode. `Stopping` = in-place walk settle to neutral, then [`Stand`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocomotionMode {
    #[default]
    Stand,
    Walk,
    Sprint,
    Stopping,
    Air,
}

impl LocomotionMode {
    pub fn uses_walk_clip(self) -> bool {
        matches!(self, Self::Walk | Self::Stopping)
    }

    pub fn uses_loco_clip(self) -> bool {
        matches!(self, Self::Walk | Self::Sprint | Self::Stopping)
    }

    pub fn is_sprint(self) -> bool {
        matches!(self, Self::Sprint)
    }

    pub fn is_air(self) -> bool {
        matches!(self, Self::Air)
    }
}

/// Blaster class (021). Secondary may only hold launcher or pistol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponClass {
    Launcher,
    Pistol,
    Smg,
    AssaultRifle,
    SniperRifle,
    Shotgun,
}

impl WeaponClass {
    pub fn from_letter(letter: u8) -> Option<Self> {
        Some(match letter {
            b'a' => Self::Launcher,
            b'b' | b'i' => Self::Pistol,
            b'c' | b'g' | b'h' | b'l' | b'm' | b'p' => Self::Smg,
            b'd' | b'n' | b'q' | b'r' => Self::AssaultRifle,
            b'e' | b'f' => Self::SniperRifle,
            b'j' | b'k' | b'o' => Self::Shotgun,
            _ => return None,
        })
    }

    pub fn allowed_in_secondary(self) -> bool {
        matches!(self, Self::Launcher | Self::Pistol)
    }
}

/// Which loadout slot is in hand (021). Unarmed = active slot empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveWeapon {
    #[default]
    Primary,
    Secondary,
}
