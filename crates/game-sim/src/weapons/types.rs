//! Weapon fire types and constants.

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

/// Resting hold sway bands (041). Amplitudes in degrees; slight by design.
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

    /// Magazine / tube capacity for this blaster (074).
    ///
    /// Semi = muzzle-load: darts in the front tubes (not a strip mag).
    /// Full-auto / burst keep class magazine sizes (058).
    pub fn mag_capacity(self) -> u16 {
        match self.mode {
            FireMode::Semi => {
                let muzzles = u16::from(muzzle_count_for_letter(self.letter).max(1));
                let pellets = u16::from(self.pellets.max(1));
                match self.muzzle_policy {
                    MuzzlePolicy::Single => pellets,
                    MuzzlePolicy::Alternate => muzzles,
                    MuzzlePolicy::All => muzzles.saturating_mul(pellets),
                }
            }
            FireMode::FullAuto | FireMode::Burst => crate::mag_capacity_for_class(self.class),
        }
    }
}

/// Kit muzzle counts for letters `a`..=`r` (must match client `BLASTER_MUZZLE_POINTS`).
pub fn muzzle_count_for_letter(letter: u8) -> u8 {
    const COUNTS: [u8; 18] = [
        1, // a
        1, // b
        1, // c
        1, // d
        1, // e
        1, // f
        1, // g
        1, // h
        2, // i
        2, // j
        1, // k
        2, // l
        1, // m
        1, // n
        4, // o
        2, // p
        2, // q
        1, // r
    ];
    letter
        .checked_sub(b'a')
        .and_then(|i| COUNTS.get(i as usize).copied())
        .unwrap_or(1)
}

/// Sprint→fire base tax before letter `T_ready` (038).
pub const SPRINT_FIRE_BASE_S: f32 = 0.12;

/// Projectile gravity (m/s²).
pub const PROJECTILE_GRAVITY: glam::Vec3 = glam::Vec3::new(0.0, -9.81, 0.0);
