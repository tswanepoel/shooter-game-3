//! Loadout equip helpers for active blaster letter.

use crate::{ActiveWeapon, SelfState, WeaponClass};

/// Equip a letter into the active slot, flipping primary↔secondary when 021 requires it.
///
/// Returns `Ok(true)` if loadout changed. Pays ready via [`FireState::sync_active_letter`]
/// on the next tick if letter changes.
pub fn equip_blaster_letter(state: &mut SelfState, letter: u8) -> Result<bool, &'static str> {
    if !state.alive {
        return Err("dead");
    }
    let class = WeaponClass::from_letter(letter).ok_or("unknown blaster letter")?;
    state.clear_emote();
    let before = (
        state.primary,
        state.secondary,
        state.active,
        state.active_blaster(),
    );

    // Prefer keeping current active slot if class fits.
    let fits_secondary = class.allowed_in_secondary();
    match state.active {
        ActiveWeapon::Primary => {
            state.set_primary(Some(letter))?;
        }
        ActiveWeapon::Secondary => {
            if fits_secondary {
                state.set_secondary(Some(letter))?;
            } else {
                // Flip to primary and equip there.
                state.active = ActiveWeapon::Primary;
                state.set_primary(Some(letter))?;
            }
        }
    }

    let after = (
        state.primary,
        state.secondary,
        state.active,
        state.active_blaster(),
    );
    Ok(before != after)
}
