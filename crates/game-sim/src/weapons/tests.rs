//! Weapon table unit tests.

use super::*;
use crate::{ammo_for_class, AmmoKind, WeaponClass};

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
    assert_eq!(weapon_def(b'b').unwrap().mag_capacity(), 1);
    assert_eq!(weapon_def(b'a').unwrap().mag_capacity(), 1);
    assert_eq!(weapon_def(b'e').unwrap().mag_capacity(), 1);
    assert_eq!(weapon_def(b'i').unwrap().mag_capacity(), 2);
    assert_eq!(weapon_def(b'j').unwrap().mag_capacity(), 6);
    assert_eq!(weapon_def(b'k').unwrap().mag_capacity(), 6);
    assert_eq!(weapon_def(b'o').unwrap().mag_capacity(), 8);
    assert_eq!(weapon_def(b'p').unwrap().mag_capacity(), 30);
}

#[test]
fn shot_interval_from_rpm() {
    let p = weapon_def(b'p').unwrap();
    let dt = p.shot_interval_s();
    assert!((dt - 60.0 / 780.0).abs() < 1e-6);
}
