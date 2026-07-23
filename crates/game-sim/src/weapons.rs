//! Baked weapon fire table (038/040).

use crate::WeaponClass;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponKick {
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub back_m: f32,
    pub settle_s: f32,
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
    pub kick: WeaponKick,
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
}

/// Sprint→fire base tax before letter `T_ready` (038).
pub const SPRINT_FIRE_BASE_S: f32 = 0.12;

/// Projectile gravity (m/s²).
pub const PROJECTILE_GRAVITY: glam::Vec3 = glam::Vec3::new(0.0, -9.81, 0.0);

fn kick_for(class: WeaponClass) -> WeaponKick {
    match class {
        WeaponClass::Pistol => WeaponKick {
            pitch_deg: 0.4,
            yaw_deg: 0.1,
            back_m: 0.008,
            settle_s: 0.04,
        },
        WeaponClass::Smg => WeaponKick {
            pitch_deg: 0.25,
            yaw_deg: 0.08,
            back_m: 0.006,
            settle_s: 0.03,
        },
        WeaponClass::AssaultRifle => WeaponKick {
            pitch_deg: 0.35,
            yaw_deg: 0.1,
            back_m: 0.010,
            settle_s: 0.045,
        },
        WeaponClass::SniperRifle => WeaponKick {
            pitch_deg: 0.7,
            yaw_deg: 0.12,
            back_m: 0.014,
            settle_s: 0.07,
        },
        WeaponClass::Shotgun => WeaponKick {
            pitch_deg: 0.9,
            yaw_deg: 0.2,
            back_m: 0.018,
            settle_s: 0.06,
        },
        WeaponClass::Launcher => WeaponKick {
            pitch_deg: 1.1,
            yaw_deg: 0.15,
            back_m: 0.020,
            settle_s: 0.08,
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
        kick: WeaponKick {
            pitch_deg: 1.1,
            yaw_deg: 0.15,
            back_m: 0.020,
            settle_s: 0.08,
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
        kick: WeaponKick {
            pitch_deg: 0.4,
            yaw_deg: 0.1,
            back_m: 0.008,
            settle_s: 0.04,
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
        kick: WeaponKick {
            pitch_deg: 0.25,
            yaw_deg: 0.08,
            back_m: 0.006,
            settle_s: 0.03,
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
        kick: WeaponKick {
            pitch_deg: 0.35,
            yaw_deg: 0.1,
            back_m: 0.010,
            settle_s: 0.045,
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
        kick: WeaponKick {
            pitch_deg: 0.7,
            yaw_deg: 0.12,
            back_m: 0.014,
            settle_s: 0.07,
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
        kick: WeaponKick {
            pitch_deg: 0.7,
            yaw_deg: 0.12,
            back_m: 0.014,
            settle_s: 0.07,
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
        kick: WeaponKick {
            pitch_deg: 0.25,
            yaw_deg: 0.08,
            back_m: 0.006,
            settle_s: 0.03,
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
        kick: WeaponKick {
            pitch_deg: 0.25,
            yaw_deg: 0.08,
            back_m: 0.006,
            settle_s: 0.03,
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
        kick: WeaponKick {
            pitch_deg: 0.4,
            yaw_deg: 0.1,
            back_m: 0.008,
            settle_s: 0.04,
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
        kick: WeaponKick {
            pitch_deg: 0.9,
            yaw_deg: 0.2,
            back_m: 0.018,
            settle_s: 0.06,
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
        kick: WeaponKick {
            pitch_deg: 0.9,
            yaw_deg: 0.2,
            back_m: 0.018,
            settle_s: 0.06,
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
        kick: WeaponKick {
            pitch_deg: 0.25,
            yaw_deg: 0.08,
            back_m: 0.006,
            settle_s: 0.03,
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
        kick: WeaponKick {
            pitch_deg: 0.25,
            yaw_deg: 0.08,
            back_m: 0.006,
            settle_s: 0.03,
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
        kick: WeaponKick {
            pitch_deg: 0.35,
            yaw_deg: 0.1,
            back_m: 0.010,
            settle_s: 0.045,
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
        kick: WeaponKick {
            pitch_deg: 0.9,
            yaw_deg: 0.2,
            back_m: 0.018,
            settle_s: 0.06,
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
        kick: WeaponKick {
            pitch_deg: 0.25,
            yaw_deg: 0.08,
            back_m: 0.006,
            settle_s: 0.03,
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
        kick: WeaponKick {
            pitch_deg: 0.35,
            yaw_deg: 0.1,
            back_m: 0.010,
            settle_s: 0.045,
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
        kick: WeaponKick {
            pitch_deg: 0.35,
            yaw_deg: 0.1,
            back_m: 0.010,
            settle_s: 0.045,
        },
    },
];

/// Look up a letter's weapon def (`a`…`r`).
pub fn weapon_def(letter: u8) -> Option<&'static WeaponDef> {
    let i = (letter as usize).checked_sub(b'a' as usize)?;
    WEAPON_TABLE.get(i).filter(|d| d.letter == letter)
}

pub fn class_kick(class: WeaponClass) -> WeaponKick {
    kick_for(class)
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
    }

    #[test]
    fn shot_interval_from_rpm() {
        let p = weapon_def(b'p').unwrap();
        let dt = p.shot_interval_s();
        assert!((dt - 60.0 / 780.0).abs() < 1e-6);
    }
}
