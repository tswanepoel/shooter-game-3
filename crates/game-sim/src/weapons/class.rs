//! Class-level defaults for ready, mode, impulse, and sway.

use crate::WeaponClass;

use super::types::{FireImpulseSize, FireMode, WeaponSway};

pub(super) fn fire_impulse_for(class: WeaponClass) -> FireImpulseSize {
    match class {
        WeaponClass::Pistol => FireImpulseSize {
            pitch_deg: 0.55,
            yaw_deg: 0.14,
            back_m: 0.009,
            fall_s: 0.05,
        },
        WeaponClass::Smg => FireImpulseSize {
            pitch_deg: 0.50,
            yaw_deg: 0.14,
            back_m: 0.007,
            fall_s: 0.055,
        },
        WeaponClass::AssaultRifle => FireImpulseSize {
            pitch_deg: 0.55,
            yaw_deg: 0.14,
            back_m: 0.010,
            fall_s: 0.055,
        },
        WeaponClass::SniperRifle => FireImpulseSize {
            pitch_deg: 0.85,
            yaw_deg: 0.16,
            back_m: 0.013,
            fall_s: 0.055,
        },
        WeaponClass::Shotgun => FireImpulseSize {
            pitch_deg: 1.05,
            yaw_deg: 0.26,
            back_m: 0.018,
            fall_s: 0.055,
        },
        WeaponClass::Launcher => FireImpulseSize {
            pitch_deg: 1.15,
            yaw_deg: 0.18,
            back_m: 0.020,
            fall_s: 0.055,
        },
    }
}

pub(super) fn sway_for(class: WeaponClass) -> WeaponSway {
    match class {
        // Busier hold.
        WeaponClass::Smg => WeaponSway {
            breath_amp_deg: 0.14,
            breath_hz: 0.17,
            tremor_amp_deg: 0.006,
            tremor_hz: 2.5,
            drift_amp_deg: 0.09,
            drift_tau_s: 6.0,
        },
        WeaponClass::Pistol => WeaponSway {
            breath_amp_deg: 0.12,
            breath_hz: 0.15,
            tremor_amp_deg: 0.005,
            tremor_hz: 2.3,
            drift_amp_deg: 0.075,
            drift_tau_s: 6.5,
        },
        WeaponClass::AssaultRifle => WeaponSway {
            breath_amp_deg: 0.10,
            breath_hz: 0.14,
            tremor_amp_deg: 0.004,
            tremor_hz: 2.1,
            drift_amp_deg: 0.065,
            drift_tau_s: 7.0,
        },
        // Near-frozen.
        WeaponClass::SniperRifle => WeaponSway {
            breath_amp_deg: 0.045,
            breath_hz: 0.11,
            tremor_amp_deg: 0.002,
            tremor_hz: 1.9,
            drift_amp_deg: 0.03,
            drift_tau_s: 8.5,
        },
        // Heavier, slower.
        WeaponClass::Shotgun => WeaponSway {
            breath_amp_deg: 0.13,
            breath_hz: 0.13,
            tremor_amp_deg: 0.004,
            tremor_hz: 2.0,
            drift_amp_deg: 0.10,
            drift_tau_s: 7.5,
        },
        WeaponClass::Launcher => WeaponSway {
            breath_amp_deg: 0.15,
            breath_hz: 0.12,
            tremor_amp_deg: 0.0035,
            tremor_hz: 1.8,
            drift_amp_deg: 0.11,
            drift_tau_s: 8.0,
        },
    }
}

pub(super) fn t_ready(class: WeaponClass) -> f32 {
    match class {
        WeaponClass::Pistol => 0.06,
        WeaponClass::Smg => 0.08,
        WeaponClass::Shotgun => 0.10,
        WeaponClass::AssaultRifle => 0.12,
        WeaponClass::SniperRifle => 0.16,
        WeaponClass::Launcher => 0.18,
    }
}

pub(super) fn mode_for(class: WeaponClass) -> FireMode {
    match class {
        WeaponClass::Smg => FireMode::FullAuto,
        WeaponClass::AssaultRifle => FireMode::Burst,
        _ => FireMode::Semi,
    }
}

pub fn class_fire_impulse(class: WeaponClass) -> FireImpulseSize {
    fire_impulse_for(class)
}

pub fn class_sway(class: WeaponClass) -> WeaponSway {
    sway_for(class)
}

/// Class-level ready time helper.
pub fn class_t_ready(class: WeaponClass) -> f32 {
    t_ready(class)
}

/// Class fire mode helper.
pub fn class_fire_mode(class: WeaponClass) -> FireMode {
    mode_for(class)
}
