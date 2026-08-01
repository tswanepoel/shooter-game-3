use glam::Vec3;

use super::*;
use crate::weapons::weapon_def;
use crate::{ActiveWeapon, AmmoKind, SelfState};

fn armed_self() -> SelfState {
    SelfState::default_loadout()
}

fn eye() -> Vec3 {
    Vec3::new(0.0, 1.52, 0.27)
}

fn muzzles() -> Vec<Vec3> {
    vec![Vec3::new(0.0, 1.4, 0.4)]
}

/// Instantly seat a spring chamber from mag (skip equip pump for fire tests).
fn seat_spring(s: &mut SelfState) {
    let _ = s.feed_chamber_from_mag(1);
}

#[test]
fn dead_does_not_fire() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    s.apply_damage(crate::HEALTH_MAX);
    assert!(!s.alive);
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    assert!(d.is_empty());
}

#[test]
fn semi_fires_once_per_edge() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;

    let m = muzzles();
    // press
    let d0 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d0.len(), 1);
    assert_eq!(d0[0].projectiles.len(), 1);
    // hold
    let d1 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert!(d1.is_empty());
    // release + press after cooldown
    let _ = fire.tick(1.0, &mut s, false, 0, eye(), &m);
    fire.cooldown_s = 0.0;
    let d2 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d2.len(), 1);
}

#[test]
fn full_auto_while_held() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    // p is SMG alternate
    fire.pay_ready(b'p');
    fire.ready_s = 0.0;
    let m = vec![Vec3::ZERO, Vec3::X];
    let mut total = 0;
    // Hold for ~0.1s at 780 RPM → interval ~0.077s → about 2 shots
    for _ in 0..20 {
        let d = fire.tick(0.01, &mut s, true, 0, eye(), &m);
        total += d.len();
    }
    assert!(total >= 2, "total discharges={total}");
}

#[test]
fn burst_three_and_blocks_side() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'd')).unwrap();
    fire.pay_ready(b'd');
    fire.ready_s = 0.0;
    let m = muzzles();
    let d0 = fire.tick(0.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d0.len(), 1);
    assert!(fire.blocks_weapon_side());
    // finish string
    let mut n = 1;
    for _ in 0..20 {
        fire.cooldown_s = 0.0;
        let d = fire.tick(0.001, &mut s, false, 0, eye(), &m);
        n += d.len();
        if !fire.burst_active() {
            break;
        }
    }
    assert_eq!(n, 3);
    assert!(!fire.blocks_weapon_side());
}

#[test]
fn burst_hold_does_not_chain_a_second_string() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'd')).unwrap();
    fire.pay_ready(b'd');
    fire.ready_s = 0.0;
    let m = muzzles();
    let mut n = 0;
    // Trigger held down throughout: one string only.
    for _ in 0..40 {
        fire.cooldown_s = 0.0;
        n += fire.tick(0.001, &mut s, true, 0, eye(), &m).len();
    }
    assert_eq!(n, 3);
    assert!(!fire.burst_active());

    // Release and press again: a fresh string.
    let _ = fire.tick(0.001, &mut s, false, 0, eye(), &m);
    fire.cooldown_s = 0.0;
    let d = fire.tick(0.001, &mut s, true, 0, eye(), &m);
    assert_eq!(d.len(), 1);
}

#[test]
fn reload_takes_time_and_blocks_fire() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'c')).unwrap();
    s.primary_mag = 0;
    fire.pay_ready(b'c');
    fire.ready_s = 0.0;
    let m = muzzles();

    assert_eq!(fire.begin_reload(&s), Some(b'c'));
    assert!(fire.loading());
    // A second ask while loading is a no-op.
    assert_eq!(fire.begin_reload(&s), None);

    // Mid-reload: rounds have not landed and the trigger does nothing.
    let _ = fire.tick(RELOAD_MAG_S * 0.5, &mut s, true, 0, eye(), &m);
    assert_eq!(s.primary_mag, 0);

    let _ = fire.tick(RELOAD_MAG_S, &mut s, false, 0, eye(), &m);
    assert!(!fire.loading());
    // Mag-fed letters instant-seat one into chamber after R lands.
    assert_eq!(
        s.primary_mag + s.primary_chamber,
        s.active_mag_capacity().unwrap()
    );
    assert_eq!(s.primary_chamber, 1);

    let d = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d.len(), 1);
}

#[test]
fn reload_needs_room_and_reserve() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'c')).unwrap();
    // Full magazine.
    s.primary_mag = s.active_mag_capacity().unwrap();
    assert_eq!(fire.begin_reload(&s), None);
    // Empty reserve.
    s.primary_mag = 0;
    s.reserve.light_foam = 0;
    assert_eq!(fire.begin_reload(&s), None);
    assert!(!fire.loading());
}

#[test]
fn fire_clears_sprint_and_taxes() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    s.apply_move(0.05, 1.0, 0.0, true);
    assert!(s.sprint_latched);
    let m = muzzles();
    let _ = fire.tick(0.0, &mut s, true, 0, eye(), &m);
    assert!(!s.sprint_latched);
    assert!(fire.sprint_fire_s > 0.1);
}

#[test]
fn projectile_falls_under_gravity() {
    let mut world = ProjectileWorld::new();
    world.spawn(Projectile {
        id: 1,
        owner: 0,
        weapon: b'b',
        ammo: AmmoKind::LightFoam,
        origin: Vec3::ZERO,
        position: Vec3::ZERO,
        velocity: Vec3::new(0.0, 0.0, 100.0),
        traveled: 0.0,
        max_range: 1000.0,
        muzzle_index: 0,
    });
    world.tick(1.0);
    let p = &world.projectiles[0];
    assert!(p.velocity.y < 0.0);
    assert!(p.position.y < 0.0);
    assert!(p.traveled > 0.0);
}

#[test]
fn despawn_at_max_range() {
    let mut world = ProjectileWorld::new();
    world.spawn(Projectile {
        id: 1,
        owner: 0,
        weapon: b'b',
        ammo: AmmoKind::LightFoam,
        origin: Vec3::ZERO,
        position: Vec3::ZERO,
        velocity: Vec3::new(0.0, 0.0, 50.0),
        traveled: 0.0,
        max_range: 10.0,
        muzzle_index: 0,
    });
    world.tick(1.0); // travels 50m > 10
    assert!(world.projectiles.is_empty());
}

#[test]
fn projectiles_spawn_from_look_not_muzzle() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    let barrel = muzzles();
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &barrel);
    assert_eq!(d.len(), 1);
    let p = &d[0].projectiles[0];
    assert!(
        (p.origin - eye()).length() < 1e-5,
        "combat origin is camera, got {:?}",
        p.origin
    );
    assert!(
        (p.origin - barrel[0]).length() > 0.1,
        "must not spawn at barrel"
    );
    assert_eq!(d[0].fired_muzzles, vec![0], "flash still names a muzzle");
}

#[test]
fn combat_spawns_without_muzzle_fx_points() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &[]);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].projectiles.len(), 1);
    assert!((d[0].projectiles[0].origin - eye()).length() < 1e-5);
    assert!(d[0].fired_muzzles.is_empty());
}

#[test]
fn spawn_carries_ammo_and_blaster_muzzle_vel() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    // Pistol b → light foam, muzzle_vel from letter.
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    let def = weapon_def(b'b').unwrap();
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    assert_eq!(d.len(), 1);
    let p = &d[0].projectiles[0];
    assert_eq!(p.ammo, AmmoKind::LightFoam);
    assert_eq!(p.weapon, b'b');
    assert!((p.velocity.length() - def.muzzle_vel).abs() < 1e-2);
    // Mass is looked up from ammo, not invented on the projectile bag.
    assert_eq!(crate::ammo_def(p.ammo).mass, crate::MASS_LIGHT_FOAM_KG);

    // Sniper e → thick foam, own muzzle speed.
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'e')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'e');
    fire.ready_s = 0.0;
    let def_e = weapon_def(b'e').unwrap();
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    let p = &d[0].projectiles[0];
    assert_eq!(p.ammo, AmmoKind::ThickFoam);
    assert!((p.velocity.length() - def_e.muzzle_vel).abs() < 1e-2);

    // Launcher a → grenade (no-mag; chamber holds the round).
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'a')).unwrap();
    s.reserve.grenade = 2;
    s.primary_chamber = 1;
    fire.pay_ready(b'a');
    fire.ready_s = 0.0;
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    assert_eq!(d[0].projectiles[0].ammo, AmmoKind::Grenade);
}

#[test]
fn shotgun_pellets_share_ammo_kind() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'k')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'k');
    fire.ready_s = 0.0;
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    assert_eq!(d[0].projectiles.len(), 6);
    for p in &d[0].projectiles {
        assert_eq!(p.ammo, AmmoKind::LightFoam);
    }
}

#[test]
fn equip_flips_to_primary_for_rifle_on_secondary() {
    let mut s = SelfState::default_loadout();
    s.active = ActiveWeapon::Secondary;
    assert_eq!(s.active_blaster(), Some(b'b'));
    equip_blaster_letter(&mut s, b'p').unwrap();
    assert_eq!(s.active, ActiveWeapon::Primary);
    assert_eq!(s.primary, Some(b'p'));
}

#[test]
fn shotgun_k_spawns_six_pellets() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'k')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'k');
    fire.ready_s = 0.0;
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    assert_eq!(d[0].projectiles.len(), 6);
}

#[test]
fn grip_bore_travels_with_fire_residual() {
    let mut s = armed_self();
    let def = weapon_def(b'b').unwrap();
    assert_eq!(s.grip_bore_m, 0.0);
    s.apply_fire_impulse(def, 1.0);
    assert!(s.grip_bore_m > 0.0, "bore={}", s.grip_bore_m);
    assert!(s.fire_fold_total() > 0.0);
    assert!(s.hip_fire_fold > 0.0 && s.shoulder_fire_fold > 0.0 && s.neck_fire_fold > 0.0);
    let bore_after = s.grip_bore_m;
    for _ in 0..120 {
        s.tick_joint_residual(1.0 / 60.0, false);
    }
    assert!(
        s.grip_bore_m < bore_after * 0.05,
        "bore did not fall: {}",
        s.grip_bore_m
    );
}

#[test]
fn fire_adds_body_residual_and_settles() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    let m = muzzles();
    assert_eq!(s.fire_fold_total(), 0.0);
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d.len(), 1);
    let fold_after = s.fire_fold_total();
    assert!(fold_after > 0.0, "fire fold={fold_after}");
    assert!(s.hip_fire_fold > 0.0 && s.neck_fire_fold > 0.0);
    for _ in 0..120 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &m);
    }
    assert!(
        s.fire_fold_total() < fold_after * 0.05,
        "fire residual did not settle: {}",
        s.fire_fold_total()
    );
}

#[test]
fn full_auto_fire_residual_stacks_climb() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'c')).unwrap();
    fire.pay_ready(b'c');
    fire.ready_s = 0.0;
    let m = muzzles();
    let one = weapon_def(b'c')
        .unwrap()
        .fire_impulse
        .pitch_deg
        .to_radians();
    let mut shots = 0u32;
    let mut peak = 0.0f32;
    let dt = 1.0 / 60.0;
    for _ in 0..45 {
        let d = fire.tick(dt, &mut s, true, 0, eye(), &m);
        shots += d.len() as u32;
        peak = peak.max(s.fire_fold_total());
    }
    assert!(shots >= 4, "expected several SMG shots, got {shots}");
    assert!(
        peak > one * 1.5,
        "fire residual should climb under spray: peak={peak} one={one} shots={shots}"
    );
}

#[test]
fn fire_fall_slows_while_fire_continues() {
    let mut s = armed_self();
    let def = weapon_def(b'c').unwrap();
    s.apply_fire_impulse(def, 1.0);
    let base = s.fire_fall_eff_s(false);
    let cont = s.fire_fall_eff_s(true);
    assert!(
        cont > base,
        "continue fall {cont} should exceed base {base}"
    );
    assert!((cont / base - 6.0).abs() < 1e-3);
}

#[test]
fn unarmed_keeps_hit_residual() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    fire.add_hit_impulse(&mut s, 20.0);
    assert!(s.hit_fold_total() > 0.0);
    s.set_primary(None).unwrap();
    s.set_secondary(None).unwrap();
    let hit_before = s.hit_fold_total();
    let _ = fire.tick(0.0, &mut s, false, 0, eye(), &[]);
    assert!(
        (s.hit_fold_total() - hit_before).abs() < 1e-5,
        "hit residual cleared on unarmed: before={hit_before} after={}",
        s.hit_fold_total()
    );
    assert_eq!(s.fire_fold_total(), 0.0);
    assert_eq!(s.grip_bore_m, 0.0);
}

#[test]
fn shots_use_weapon_line_from_fire_residual() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    s.hip_fire_fold = 3f32.to_radians();
    s.shoulder_fire_fold = 7f32.to_radians();
    s.neck_fire_fold = 5f32.to_radians();
    s.compose_joints();
    s.fire_fall_s = 1000.0;
    let expected = s.weapon_line().expect("armed");
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    let vel = d[0].projectiles[0].velocity.normalize();
    assert!(
        vel.dot(expected) > 0.995,
        "vel={vel} expected≈{expected} dot={}",
        vel.dot(expected)
    );
    assert!(
        vel.y > 0.1,
        "fire residual fold should lift aim, vel.y={}",
        vel.y
    );
}

#[test]
fn armed_hold_advances_sway_on_shoulder() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    assert_eq!(s.shoulder_sway_fold, 0.0);
    assert_eq!(s.shoulder_sway_twist, 0.0);
    for _ in 0..90 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &muzzles());
    }
    let mag = s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs();
    assert!(mag > 1e-5, "sway should move while armed hold, mag={mag}");
}

#[test]
fn unarmed_clears_sway_and_residual() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'p')).unwrap();
    fire.pay_ready(b'p');
    fire.ready_s = 0.0;
    for _ in 0..60 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
    }
    assert!(s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs() > 0.0);
    s.set_primary(None).unwrap();
    s.set_secondary(None).unwrap();
    let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
    assert_eq!(s.shoulder_sway_fold, 0.0);
    assert_eq!(s.shoulder_sway_twist, 0.0);
    assert_eq!(s.fire_fold_total(), 0.0);
    assert_eq!(s.hip_fire_fold, 0.0);
    assert_eq!(s.neck_fire_fold, 0.0);
    assert!(s.weapon_line().is_none());
}

#[test]
fn shots_use_weapon_line_with_sway() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    for _ in 0..120 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
    }
    assert_eq!(s.fire_fold_total(), 0.0);
    s.compose_joints();
    let expected = s.weapon_line().expect("armed");
    assert!(
        s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs() > 1e-5,
        "expected nonzero sway on shoulder"
    );
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    let vel = d[0].projectiles[0].velocity.normalize();
    // Spread on pistol is small; should still align roughly with weapon line.
    assert!(
        vel.dot(expected) > 0.98,
        "vel={vel} expected≈{expected} dot={}",
        vel.dot(expected)
    );
}

#[test]
fn hit_impulse_from_damage_and_settles() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    assert_eq!(s.hit_fold_total(), 0.0);
    let dmg = crate::impact_damage(AmmoKind::LightFoam, 400.0, crate::HitBodyPart::Torso);
    assert!(dmg > 0.0);
    fire.add_hit_impulse(&mut s, dmg);
    let fold = s.hit_fold_total();
    assert!(fold > 0.0, "hit fold={fold}");
    assert!(s.hip_hit_fold > 0.0 && s.shoulder_hit_fold > 0.0 && s.neck_hit_fold > 0.0);
    // Stronger impact → stronger residual (within cap).
    let mut s2 = armed_self();
    fire.add_hit_impulse(
        &mut s2,
        crate::impact_damage(AmmoKind::Grenade, 400.0, crate::HitBodyPart::Torso),
    );
    assert!(s2.hit_fold_total() > fold);
    // Zero damage: no impulse.
    let mut s0 = armed_self();
    fire.add_hit_impulse(&mut s0, 0.0);
    assert_eq!(s0.hit_fold_total(), 0.0);
    // Settles.
    for _ in 0..120 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &muzzles());
    }
    assert!(
        s.hit_fold_total() < fold * 0.05,
        "hit residual did not settle: {}",
        s.hit_fold_total()
    );
}

#[test]
fn shots_use_weapon_line_with_hit_residual() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    fire.add_hit_impulse(
        &mut s,
        crate::impact_damage(AmmoKind::LightFoam, 400.0, crate::HitBodyPart::Torso),
    );
    s.hip_fire_fold = 0.0;
    s.shoulder_fire_fold = 0.0;
    s.shoulder_fire_twist = 0.0;
    s.neck_fire_fold = 0.0;
    s.hit_fall_s = 1000.0;
    s.compose_joints();
    let expected = s.weapon_line().expect("armed");
    assert!(s.hit_fold_total() > 0.0);
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    let vel = d[0].projectiles[0].velocity.normalize();
    assert!(
        vel.dot(expected) > 0.98,
        "vel={vel} expected≈{expected} dot={}",
        vel.dot(expected)
    );
    assert!(vel.y > 0.0, "hit fold should lift aim, vel.y={}", vel.y);
}

#[test]
fn sniper_sway_quieter_than_smg() {
    fn peak_sway(letter: u8) -> f32 {
        let mut fire = FireState::new();
        let mut s = armed_self();
        s.set_primary(Some(letter)).unwrap();
        fire.pay_ready(letter);
        fire.ready_s = 0.0;
        let mut peak = 0.0f32;
        for _ in 0..600 {
            let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
            let m = s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs();
            peak = peak.max(m);
        }
        peak
    }
    let sniper = peak_sway(b'e');
    let smg = peak_sway(b'p');
    assert!(
        sniper < smg * 0.85,
        "sniper peak={sniper} should be quieter than smg={smg}"
    );
}

#[test]
fn look_rate_damps_sway() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    seat_spring(&mut s);
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    for _ in 0..90 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
    }
    let still = s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs();
    assert!(still > 1e-5);
    // Whip look hard for a stretch.
    for i in 0..30 {
        s.set_look(i as f32 * 0.4, (i as f32 * 0.05).sin() * 0.2);
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &[]);
    }
    let moving = s.shoulder_sway_fold.abs() + s.shoulder_sway_twist.abs();
    assert!(
        moving < still * 0.5,
        "look-rate should damp sway: still={still} moving={moving}"
    );
}

#[test]
fn unarmed_has_no_weapon_line() {
    let mut s = armed_self();
    s.set_primary(None).unwrap();
    s.set_secondary(None).unwrap();
    assert!(s.weapon_line().is_none());
    assert!(s.reticle_world(eye()).is_none());
}

#[test]
fn empty_mag_blocks_fire() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    s.primary_mag = 0;
    fire.pay_ready(b'b');
    fire.ready_s = 0.0;
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    assert!(d.is_empty());
}

#[test]
fn fire_spends_chamber_not_mag() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'e')).unwrap();
    seat_spring(&mut s);
    let mag_before = s.primary_mag;
    fire.pay_ready(b'e');
    fire.ready_s = 0.0;
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].projectiles.len(), 1);
    assert_eq!(s.primary_chamber, 0);
    assert_eq!(s.primary_mag, mag_before);
}

#[test]
fn spring_shotgun_one_shell_full_spray() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'k')).unwrap();
    s.primary_mag = 3;
    seat_spring(&mut s);
    assert_eq!(s.primary_mag, 2);
    assert_eq!(s.primary_chamber, 1);
    fire.pay_ready(b'k');
    fire.ready_s = 0.0;
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &muzzles());
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].projectiles.len(), 6);
    assert_eq!(s.primary_chamber, 0);
    assert_eq!(s.primary_mag, 2);
}

#[test]
fn every_firing_muzzle_spends_a_seated_round() {
    // `o`: four barrels fire together, two pellets each — four rounds leave the chamber.
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'o')).unwrap();
    s.primary_chamber = 4;
    fire.pay_ready(b'o');
    fire.ready_s = 0.0;
    let m = vec![
        Vec3::new(-0.1, 1.4, 0.4),
        Vec3::new(0.1, 1.4, 0.4),
        Vec3::new(-0.1, 1.3, 0.4),
        Vec3::new(0.1, 1.3, 0.4),
    ];
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].fired_muzzles.len(), 4);
    assert_eq!(d[0].projectiles.len(), 8);
    assert_eq!(s.primary_chamber, 0);
}

#[test]
fn part_seated_chamber_fires_only_paid_muzzles() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'o')).unwrap();
    s.primary_chamber = 2;
    s.reserve.light_foam = 0;
    fire.pay_ready(b'o');
    fire.ready_s = 0.0;
    let m = vec![
        Vec3::new(-0.1, 1.4, 0.4),
        Vec3::new(0.1, 1.4, 0.4),
        Vec3::new(-0.1, 1.3, 0.4),
        Vec3::new(0.1, 1.3, 0.4),
    ];
    let d = fire.tick(0.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d[0].fired_muzzles.len(), 2);
    assert_eq!(d[0].projectiles.len(), 4);
    assert_eq!(s.primary_chamber, 0);
}

#[test]
fn no_mag_alternate_fires_each_seat_before_a_pump() {
    // `i`: chamber 2, one barrel per press — both go before the chamber refills.
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'i')).unwrap();
    s.primary_chamber = 2;
    s.reserve.light_foam = 10;
    fire.pay_ready(b'i');
    fire.ready_s = 0.0;
    let m = vec![Vec3::new(0.0, 1.4, 0.4), Vec3::new(0.0, 1.3, 0.4)];

    let d0 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d0.len(), 1);
    assert_eq!(s.primary_chamber, 1);
    assert!(!fire.pumping());

    let _ = fire.tick(1.0, &mut s, false, 0, eye(), &m);
    fire.cooldown_s = 0.0;
    let d1 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d1.len(), 1);
    assert_eq!(d1[0].fired_muzzles, vec![1]);
    assert_eq!(s.primary_chamber, 0);
    assert!(fire.pumping());
}

#[test]
fn reload_moves_reserve_into_mag() {
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    s.primary_mag = 0;
    s.reserve.light_foam = 5;
    assert!(s.try_reload());
    assert_eq!(s.primary_mag, 5);
    assert_eq!(s.reserve.light_foam, 0);
    // Full mag: no more.
    s.primary_mag = s.active_mag_capacity().unwrap();
    s.reserve.light_foam = 10;
    assert!(!s.try_reload());
}

#[test]
fn spring_mag_pumps_between_shots_without_taking_reserve() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'e')).unwrap();
    seat_spring(&mut s);
    s.reserve.thick_foam = 30;
    fire.pay_ready(b'e');
    fire.ready_s = 0.0;

    let m = muzzles();
    let mag_before = s.primary_mag;
    let d = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d.len(), 1);
    assert_eq!(s.primary_chamber, 0);
    assert_eq!(s.primary_mag, mag_before);
    assert!(fire.pumping());
    assert_eq!(s.reserve.thick_foam, 30);

    for _ in 0..60 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &m);
    }
    assert!(!fire.pumping());
    assert_eq!(s.primary_chamber, 1);
    assert_eq!(s.primary_mag, mag_before - 1);
    assert_eq!(s.reserve.thick_foam, 30);
    assert_eq!(
        fire.take_pump_cues(),
        vec![PumpCue::Start, PumpCue::Seat, PumpCue::End]
    );
}

#[test]
fn spring_mag_empty_needs_r_not_auto_reserve() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'e')).unwrap();
    s.primary_mag = 1;
    seat_spring(&mut s);
    assert_eq!(s.primary_mag, 0);
    assert_eq!(s.primary_chamber, 1);
    s.reserve.thick_foam = 10;
    fire.pay_ready(b'e');
    fire.ready_s = 0.0;

    let m = muzzles();
    let _ = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(s.primary_chamber, 0);
    assert_eq!(s.primary_mag, 0);
    assert!(!fire.pumping());
    for _ in 0..60 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &m);
    }
    assert_eq!(s.primary_mag, 0);
    assert_eq!(s.reserve.thick_foam, 10);
    assert_eq!(fire.begin_reload(&s), Some(b'e'));
}

#[test]
fn spring_equip_auto_pumps_from_mag() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'e')).unwrap();
    assert_eq!(s.primary_chamber, 0);
    fire.pay_ready(b'e');
    fire.ready_s = 0.0;

    let m = muzzles();
    let mag_before = s.primary_mag;
    let d0 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert!(d0.is_empty());
    assert!(fire.pumping());

    for _ in 0..60 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &m);
    }
    assert!(!fire.pumping());
    assert_eq!(s.primary_chamber, 1);
    assert_eq!(s.primary_mag, mag_before - 1);
    assert_eq!(
        fire.take_pump_cues(),
        vec![PumpCue::Start, PumpCue::Seat, PumpCue::End]
    );

    let d1 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d1.len(), 1);
}

#[test]
fn no_mag_auto_pumps_from_reserve() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'i')).unwrap();
    s.reserve.light_foam = 4;
    fire.pay_ready(b'i');
    fire.ready_s = 0.0;

    let m = vec![Vec3::new(0.0, 1.4, 0.4), Vec3::new(0.0, 1.3, 0.4)];
    let d0 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert!(d0.is_empty());
    assert_eq!(s.primary_chamber, 0);
    assert!(fire.pumping());
    assert_eq!(fire.begin_reload(&s), None);

    let mut cues = Vec::new();
    for _ in 0..120 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &m);
        cues.extend(fire.take_pump_cues());
    }
    assert_eq!(
        cues,
        vec![PumpCue::Start, PumpCue::Seat, PumpCue::Seat, PumpCue::End]
    );
    assert!(!fire.pumping());
    assert_eq!(s.primary_chamber, 2);
    assert_eq!(s.reserve.light_foam, 2);

    let d1 = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(d1.len(), 1);
    assert_eq!(s.primary_chamber, 1);
}

#[test]
fn no_mag_auto_pump_needs_reserve() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'i')).unwrap();
    s.primary_chamber = 1;
    s.reserve.light_foam = 0;
    fire.pay_ready(b'i');
    fire.ready_s = 0.0;

    let m = vec![Vec3::new(0.0, 1.4, 0.4), Vec3::new(0.0, 1.3, 0.4)];
    let _ = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    assert_eq!(s.primary_chamber, 0);
    assert!(!fire.pumping());
    for _ in 0..60 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &m);
    }
    assert_eq!(s.primary_chamber, 0);
    assert!(fire.take_pump_cues().is_empty());
}

#[test]
fn full_auto_does_not_auto_pump() {
    let mut fire = FireState::new();
    let mut s = armed_self();
    s.set_primary(Some(b'p')).unwrap();
    s.primary_mag = 1;
    s.reserve.light_foam = 30;
    fire.pay_ready(b'p');
    fire.ready_s = 0.0;

    let m = muzzles();
    let _ = fire.tick(1.0 / 60.0, &mut s, true, 0, eye(), &m);
    for _ in 0..60 {
        let _ = fire.tick(1.0 / 60.0, &mut s, false, 0, eye(), &m);
    }
    assert_eq!(s.primary_mag, 0);
    assert!(fire.take_pump_cues().is_empty());
}

#[test]
fn spawn_ammo_fills_mag_load_and_draft_reserve() {
    let mut s = SelfState::default_loadout();
    s.primary = Some(b'e'); // sniper → thick foam
    s.secondary = Some(b'a'); // launcher → grenade
    s.apply_spawn_ammo();
    assert_eq!(s.primary_mag, crate::spawn_mag_for_letter(b'e'));
    assert_eq!(s.secondary_mag, crate::spawn_mag_for_letter(b'a'));
    assert_eq!(s.primary_chamber, 0);
    assert_eq!(s.secondary_chamber, 0);
    assert_eq!(s.reserve.thick_foam, crate::spawn_spare_for_letter(b'e'));
    assert_eq!(s.reserve.grenade, crate::spawn_spare_for_letter(b'a'));
    assert_eq!(s.reserve.light_foam, 0);
}

#[test]
fn dump_death_ammo_empties_reserve_keeps_mag() {
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    s.primary_mag = 4;
    s.reserve.light_foam = 6;
    s.reserve.thick_foam = 3;
    let (kind, n) = s.dump_death_ammo().expect("dump");
    assert_eq!(kind, AmmoKind::LightFoam);
    assert_eq!(n, 6);
    assert_eq!(s.primary_mag, 4);
    assert_eq!(s.reserve.light_foam, 0);
    assert_eq!(s.reserve.thick_foam, 3);
}

#[test]
fn take_active_blaster_clears_slot() {
    let mut s = armed_self();
    s.set_primary(Some(b'b')).unwrap();
    s.primary_mag = 4;
    let (letter, mag) = s.take_active_blaster_drop().expect("drop");
    assert_eq!(letter, b'b');
    assert_eq!(mag, 4);
    assert!(s.primary.is_none());
    assert_eq!(s.primary_mag, 0);
}

#[test]
fn grant_floor_blaster_fills_then_swaps() {
    let mut s = armed_self();
    s.set_primary(Some(b'p')).unwrap();
    s.primary_mag = 10;
    s.set_secondary(None).unwrap();
    // Free secondary — any class on pickup.
    let displaced = s.grant_floor_blaster(b'd', 7).unwrap();
    assert!(displaced.is_none());
    assert_eq!(s.secondary, Some(b'd'));
    assert_eq!(s.secondary_mag, 7);
    assert_eq!(s.active, ActiveWeapon::Secondary);

    // Both full — swap active (secondary). No-mag `i` seats into chamber (cap 2).
    let displaced = s.grant_floor_blaster(b'i', 5).unwrap();
    assert_eq!(displaced, Some((b'd', 7)));
    assert_eq!(s.secondary, Some(b'i'));
    assert_eq!(s.secondary_mag, 0);
    assert_eq!(s.secondary_chamber, 2);

    // Active primary — swap primary. Springer pistol clamps to mag 12.
    s.active = ActiveWeapon::Primary;
    let displaced = s.grant_floor_blaster(b'b', 3).unwrap();
    assert_eq!(displaced, Some((b'p', 10)));
    assert_eq!(s.primary, Some(b'b'));
    assert_eq!(s.primary_mag, 3);
    assert_eq!(s.active, ActiveWeapon::Primary);
}
