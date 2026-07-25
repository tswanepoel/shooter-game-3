//! Baked weapon fire table (038/040/041/042/048/049).
//!
//! Blaster owns initial velocity and ammo kind (via class). Ammo mass is on
//! [`crate::AmmoKind`]. Fire-impulse size and base fall live here; continue-fall
//! scaling is on the figure.

use crate::{ammo_for_class, AmmoKind, WeaponClass};

/// Fire trigger mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireMode {
    /// One discharge per press edge.
    Semi,
    /// While held, paced by RPM.
    FullAuto,
    /// Fixed-length string per press (AR).
    Burst,
}

/// Which kit muzzles fire on a discharge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuzzlePolicy {
    /// Pellets from the primary (first) muzzle only.
    Single,
    /// Every kit muzzle on the same discharge.
    All,
    /// Round-robin one muzzle per discharge.
    Alternate,
}

/// Fire impulse size for a blaster (fold/twist deg, grip bore m, base fall s).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireImpulseSize {
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub back_m: f32,
    pub fall_s: f32,
}

/// Resting aim-hold bands (041). Amplitudes in degrees; slight by design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponSway {
    /// Slow vertical-dominant breath amplitude (deg).
    pub breath_amp_deg: f32,
    pub breath_hz: f32,
    /// Tiny high-frequency tremor amplitude (deg).
    pub tremor_amp_deg: f32,
    pub tremor_hz: f32,
    /// Mean-reverting drift scale (deg, stationary std-ish).
    pub drift_amp_deg: f32,
    /// Drift mean-reversion time constant (s).
    pub drift_tau_s: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponDef {
    pub letter: u8,
    pub class: WeaponClass,
    pub mode: FireMode,
    /// After equip / swap onto this letter (seconds).
    pub t_ready: f32,
    pub rpm: f32,
    /// Muzzle velocity (m/s).
    pub muzzle_vel: f32,
    /// Path length before despawn (m).
    pub max_range: f32,
    /// Look-space cone half-angle (degrees).
    pub spread_half_deg: f32,
    pub muzzle_policy: MuzzlePolicy,
    /// Pellets per firing muzzle on a discharge.
    pub pellets: u8,
    /// Burst string length (meaningful for [`FireMode::Burst`]).
    pub burst_count: u8,
    pub fire_impulse: FireImpulseSize,
}

impl WeaponDef {
    /// Seconds between discharges at this letter's RPM.
    pub fn shot_interval_s(self) -> f32 {
        if self.rpm <= 0.0 {
            0.0
        } else {
            60.0 / self.rpm
        }
    }

    /// Ammo kind this blaster fires (class map; no per-letter override).
    pub fn ammo(self) -> AmmoKind {
        ammo_for_class(self.class)
    }
}

/// Sprint→fire base tax before letter `T_ready` (038).
pub const SPRINT_FIRE_BASE_S: f32 = 0.12;

/// Projectile gravity (m/s²).
pub const PROJECTILE_GRAVITY: glam::Vec3 = glam::Vec3::new(0.0, -9.81, 0.0);

fn fire_impulse_for(class: WeaponClass) -> FireImpulseSize {
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

fn sway_for(class: WeaponClass) -> WeaponSway {
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

fn t_ready(class: WeaponClass) -> f32 {
    match class {
        WeaponClass::Pistol => 0.06,
        WeaponClass::Smg => 0.08,
        WeaponClass::Shotgun => 0.10,
        WeaponClass::AssaultRifle => 0.12,
        WeaponClass::SniperRifle => 0.16,
        WeaponClass::Launcher => 0.18,
    }
}

fn mode_for(class: WeaponClass) -> FireMode {
    match class {
        WeaponClass::Smg => FireMode::FullAuto,
        WeaponClass::AssaultRifle => FireMode::Burst,
        _ => FireMode::Semi,
    }
}

pub static WEAPON_TABLE: [WeaponDef; 18] = [
    // a launcher
    WeaponDef {
        letter: b'a',
        class: WeaponClass::Launcher,
        mode: FireMode::Semi,
        t_ready: 0.18,
        rpm: 48.0,
        muzzle_vel: 85.0,
        max_range: 80.0,
        spread_half_deg: 0.5,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 1.15,
            yaw_deg: 0.18,
            back_m: 0.020,
            fall_s: 0.055,
        },
    },
    // b pistol
    WeaponDef {
        letter: b'b',
        class: WeaponClass::Pistol,
        mode: FireMode::Semi,
        t_ready: 0.06,
        rpm: 220.0,
        muzzle_vel: 380.0,
        max_range: 120.0,
        spread_half_deg: 0.4,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.55,
            yaw_deg: 0.14,
            back_m: 0.009,
            fall_s: 0.05,
        },
    },
    // c smg
    WeaponDef {
        letter: b'c',
        class: WeaponClass::Smg,
        mode: FireMode::FullAuto,
        t_ready: 0.08,
        rpm: 700.0,
        muzzle_vel: 420.0,
        max_range: 140.0,
        spread_half_deg: 0.6,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.50,
            yaw_deg: 0.14,
            back_m: 0.007,
            fall_s: 0.055,
        },
    },
    // d AR
    WeaponDef {
        letter: b'd',
        class: WeaponClass::AssaultRifle,
        mode: FireMode::Burst,
        t_ready: 0.12,
        rpm: 600.0,
        muzzle_vel: 650.0,
        max_range: 200.0,
        spread_half_deg: 0.35,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 3,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.55,
            yaw_deg: 0.14,
            back_m: 0.010,
            fall_s: 0.055,
        },
    },
    // e sniper
    WeaponDef {
        letter: b'e',
        class: WeaponClass::SniperRifle,
        mode: FireMode::Semi,
        t_ready: 0.16,
        rpm: 48.0,
        muzzle_vel: 820.0,
        max_range: 300.0,
        spread_half_deg: 0.15,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.85,
            yaw_deg: 0.16,
            back_m: 0.013,
            fall_s: 0.055,
        },
    },
    // f sniper
    WeaponDef {
        letter: b'f',
        class: WeaponClass::SniperRifle,
        mode: FireMode::Semi,
        t_ready: 0.16,
        rpm: 42.0,
        muzzle_vel: 850.0,
        max_range: 320.0,
        spread_half_deg: 0.12,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.85,
            yaw_deg: 0.16,
            back_m: 0.013,
            fall_s: 0.055,
        },
    },
    // g smg
    WeaponDef {
        letter: b'g',
        class: WeaponClass::Smg,
        mode: FireMode::FullAuto,
        t_ready: 0.08,
        rpm: 750.0,
        muzzle_vel: 400.0,
        max_range: 130.0,
        spread_half_deg: 0.65,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.50,
            yaw_deg: 0.14,
            back_m: 0.007,
            fall_s: 0.055,
        },
    },
    // h smg
    WeaponDef {
        letter: b'h',
        class: WeaponClass::Smg,
        mode: FireMode::FullAuto,
        t_ready: 0.08,
        rpm: 720.0,
        muzzle_vel: 410.0,
        max_range: 135.0,
        spread_half_deg: 0.6,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.50,
            yaw_deg: 0.14,
            back_m: 0.007,
            fall_s: 0.055,
        },
    },
    // i pistol alternate
    WeaponDef {
        letter: b'i',
        class: WeaponClass::Pistol,
        mode: FireMode::Semi,
        t_ready: 0.06,
        rpm: 200.0,
        muzzle_vel: 360.0,
        max_range: 110.0,
        spread_half_deg: 0.45,
        muzzle_policy: MuzzlePolicy::Alternate,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.55,
            yaw_deg: 0.14,
            back_m: 0.009,
            fall_s: 0.05,
        },
    },
    // j shotgun all(2) × 3
    WeaponDef {
        letter: b'j',
        class: WeaponClass::Shotgun,
        mode: FireMode::Semi,
        t_ready: 0.10,
        rpm: 90.0,
        muzzle_vel: 380.0,
        max_range: 60.0,
        spread_half_deg: 2.5,
        muzzle_policy: MuzzlePolicy::All,
        pellets: 3,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 1.05,
            yaw_deg: 0.26,
            back_m: 0.018,
            fall_s: 0.055,
        },
    },
    // k shotgun single × 6
    WeaponDef {
        letter: b'k',
        class: WeaponClass::Shotgun,
        mode: FireMode::Semi,
        t_ready: 0.10,
        rpm: 85.0,
        muzzle_vel: 370.0,
        max_range: 55.0,
        spread_half_deg: 3.0,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 6,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 1.05,
            yaw_deg: 0.26,
            back_m: 0.018,
            fall_s: 0.055,
        },
    },
    // l smg alternate
    WeaponDef {
        letter: b'l',
        class: WeaponClass::Smg,
        mode: FireMode::FullAuto,
        t_ready: 0.08,
        rpm: 680.0,
        muzzle_vel: 415.0,
        max_range: 135.0,
        spread_half_deg: 0.6,
        muzzle_policy: MuzzlePolicy::Alternate,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.50,
            yaw_deg: 0.14,
            back_m: 0.007,
            fall_s: 0.055,
        },
    },
    // m smg
    WeaponDef {
        letter: b'm',
        class: WeaponClass::Smg,
        mode: FireMode::FullAuto,
        t_ready: 0.08,
        rpm: 710.0,
        muzzle_vel: 405.0,
        max_range: 130.0,
        spread_half_deg: 0.65,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.50,
            yaw_deg: 0.14,
            back_m: 0.007,
            fall_s: 0.055,
        },
    },
    // n AR
    WeaponDef {
        letter: b'n',
        class: WeaponClass::AssaultRifle,
        mode: FireMode::Burst,
        t_ready: 0.12,
        rpm: 580.0,
        muzzle_vel: 640.0,
        max_range: 200.0,
        spread_half_deg: 0.35,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 3,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.55,
            yaw_deg: 0.14,
            back_m: 0.010,
            fall_s: 0.055,
        },
    },
    // o shotgun all(4) × 2
    WeaponDef {
        letter: b'o',
        class: WeaponClass::Shotgun,
        mode: FireMode::Semi,
        t_ready: 0.10,
        rpm: 75.0,
        muzzle_vel: 360.0,
        max_range: 50.0,
        spread_half_deg: 3.5,
        muzzle_policy: MuzzlePolicy::All,
        pellets: 2,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 1.05,
            yaw_deg: 0.26,
            back_m: 0.018,
            fall_s: 0.055,
        },
    },
    // p smg
    WeaponDef {
        letter: b'p',
        class: WeaponClass::Smg,
        mode: FireMode::FullAuto,
        t_ready: 0.08,
        rpm: 780.0,
        muzzle_vel: 430.0,
        max_range: 140.0,
        spread_half_deg: 0.55,
        muzzle_policy: MuzzlePolicy::Alternate,
        pellets: 1,
        burst_count: 0,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.50,
            yaw_deg: 0.14,
            back_m: 0.007,
            fall_s: 0.055,
        },
    },
    // q AR alternate
    WeaponDef {
        letter: b'q',
        class: WeaponClass::AssaultRifle,
        mode: FireMode::Burst,
        t_ready: 0.12,
        rpm: 560.0,
        muzzle_vel: 630.0,
        max_range: 195.0,
        spread_half_deg: 0.4,
        muzzle_policy: MuzzlePolicy::Alternate,
        pellets: 1,
        burst_count: 3,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.55,
            yaw_deg: 0.14,
            back_m: 0.010,
            fall_s: 0.055,
        },
    },
    // r AR
    WeaponDef {
        letter: b'r',
        class: WeaponClass::AssaultRifle,
        mode: FireMode::Burst,
        t_ready: 0.12,
        rpm: 590.0,
        muzzle_vel: 645.0,
        max_range: 200.0,
        spread_half_deg: 0.35,
        muzzle_policy: MuzzlePolicy::Single,
        pellets: 1,
        burst_count: 3,
        fire_impulse: FireImpulseSize {
            pitch_deg: 0.55,
            yaw_deg: 0.14,
            back_m: 0.010,
            fall_s: 0.055,
        },
    },
];

/// Look up a letter's weapon def (`a`…`r`).
pub fn weapon_def(letter: u8) -> Option<&'static WeaponDef> {
    let i = (letter as usize).checked_sub(b'a' as usize)?;
    WEAPON_TABLE.get(i).filter(|d| d.letter == letter)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_covers_a_through_r() {
        for (i, def) in WEAPON_TABLE.iter().enumerate() {
            assert_eq!(def.letter, b'a' + i as u8);
            assert_eq!(WeaponClass::from_letter(def.letter), Some(def.class));
            assert!(def.rpm > 0.0);
            assert!(def.muzzle_vel > 0.0);
            assert!(def.max_range > 0.0);
            assert!(def.pellets >= 1);
            assert_eq!(def.ammo(), ammo_for_class(def.class));
        }
        assert_eq!(weapon_def(b'p').unwrap().mode, FireMode::FullAuto);
        assert_eq!(weapon_def(b'd').unwrap().mode, FireMode::Burst);
        assert_eq!(weapon_def(b'd').unwrap().burst_count, 3);
        assert_eq!(weapon_def(b'b').unwrap().mode, FireMode::Semi);
        assert_eq!(weapon_def(b'j').unwrap().muzzle_policy, MuzzlePolicy::All);
        assert_eq!(weapon_def(b'k').unwrap().pellets, 6);
        assert_eq!(
            weapon_def(b'p').unwrap().muzzle_policy,
            MuzzlePolicy::Alternate
        );
        // Spec 042 letter → ammo samples
        assert_eq!(weapon_def(b'a').unwrap().ammo(), AmmoKind::Grenade);
        assert_eq!(weapon_def(b'e').unwrap().ammo(), AmmoKind::ThickFoam);
        assert_eq!(weapon_def(b'f').unwrap().ammo(), AmmoKind::ThickFoam);
        assert_eq!(weapon_def(b'b').unwrap().ammo(), AmmoKind::LightFoam);
        assert_eq!(weapon_def(b'k').unwrap().ammo(), AmmoKind::LightFoam);
    }

    #[test]
    fn shot_interval_from_rpm() {
        let p = weapon_def(b'p').unwrap();
        let dt = p.shot_interval_s();
        assert!((dt - 60.0 / 780.0).abs() < 1e-6);
    }
}
