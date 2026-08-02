use glam::Vec3;

use super::pose::{elevation_targets, residual_fold_split};
use super::*;

fn drive_look_dir(s: &SelfState) -> Vec3 {
    let yaw = s.look_yaw();
    let pitch = s.look_pitch();
    let cp = pitch.cos();
    Vec3::new(yaw.sin() * cp, pitch.sin(), yaw.cos() * cp)
}

#[test]
fn default_faces_plus_z_at_origin() {
    let s = SelfState::default_loadout();
    assert_eq!(s.position, Vec3::ZERO);
    assert_eq!(s.character, b'a');
    assert_eq!(s.primary, Some(b'p'));
    assert_eq!(s.secondary, Some(b'b'));
    assert_eq!(s.active, ActiveWeapon::Primary);
    assert_eq!(s.active_blaster(), Some(b'p'));
    assert!(s.is_armed());
    let f = drive_look_dir(&s);
    assert!(f.dot(Vec3::Z) > 0.99);
    assert!((f.length() - 1.0).abs() < 1e-5);
    assert_eq!(s.locomotion, LocomotionMode::Stand);
}

#[test]
fn look_snaps_body_pose_immediately() {
    let mut s = SelfState::default_loadout();
    s.apply_look(1.0 / 60.0, 1.0, 0.3);
    assert!((s.facing - s.look_yaw()).abs() < 1e-6);
    assert!((s.look_offset_yaw).abs() < 1e-6);
    assert!((s.neck_twist).abs() < 1e-6);
    let (hip_tgt, shoulder_tgt) = elevation_targets(s.look_offset_pitch);
    assert!((s.hip_fold - hip_tgt).abs() < 1e-6);
    assert!((s.shoulder_fold - shoulder_tgt).abs() < 1e-6);
}

#[test]
fn elevation_at_full_look_up() {
    let mut s = SelfState::default_loadout();
    s.look_offset_pitch = LOOK_ELEV_CAP_RAD;
    s.sync_pose();
    assert!((s.hip_fold - HIP_FOLD_OUTWARD_RAD).abs() < 1e-5);
    assert!((s.shoulder_fold - SHOULDER_FOLD_OUTWARD_RAD).abs() < 1e-5);
    let f = drive_look_dir(&s);
    assert!(f.dot(Vec3::Y) > 0.99, "forward={f}");
}

#[test]
fn look_offset_pitch_clamped_to_pm_90() {
    let mut s = SelfState::default_loadout();
    s.apply_look(1.0 / 60.0, 0.0, 10.0);
    assert!((s.look_offset_pitch - LOOK_ELEV_CAP_RAD).abs() < 1e-5);
    s.apply_look(1.0 / 60.0, 0.0, -20.0);
    assert!((s.look_offset_pitch + LOOK_ELEV_CAP_RAD).abs() < 1e-5);
}

#[test]
fn reticle_lies_on_weapon_line_when_residual_zero() {
    let s = SelfState::default_loadout();
    let eye = Vec3::new(0.0, 1.5, 0.0);
    let r = s.reticle_world(eye).expect("armed");
    let along = (r - eye).normalize();
    assert!(along.dot(drive_look_dir(&s)) > 0.99);
    assert!(((r - eye).length() - RETICLE_DEPTH_M).abs() < 1e-5);
    let wl = s.weapon_line().expect("armed");
    assert!(along.dot(wl) > 0.99);
}

#[test]
fn reticle_follows_weapon_line_with_fire_residual() {
    let mut s = SelfState::default_loadout();
    let eye = Vec3::new(0.0, 1.5, 0.0);
    let fold = 5f32.to_radians();
    let (h, sh, n) = residual_fold_split(fold);
    s.hip_fire_fold = h;
    s.shoulder_fire_fold = sh;
    s.neck_fire_fold = n;
    s.compose_joints();
    let r = s.reticle_world(eye).expect("armed");
    let along = (r - eye).normalize();
    let wl = s.weapon_line().expect("armed");
    assert!(along.dot(wl) > 0.99);
    assert!(along.dot(drive_look_dir(&s)) < 0.999);
    assert!(s.shoulder_fold > elevation_targets(s.look_offset_pitch).1);
    assert!(s.hip_fold > elevation_targets(s.look_offset_pitch).0);
    assert!(s.neck_fold > 0.0);
}

#[test]
fn fire_impulse_splits_fold_across_hip_shoulder_neck() {
    let mut s = SelfState::default_loadout();
    let def = crate::weapons::weapon_def(b'b').unwrap();
    let total = def.fire_impulse.pitch_deg.to_radians();
    s.apply_fire_impulse(def, 1.0);
    let sum = s.fire_fold_total();
    assert!((sum - total).abs() < 1e-5, "sum={sum} total={total}");
    assert!(s.hip_fire_fold > 0.0);
    assert!(s.shoulder_fire_fold > 0.0);
    assert!(s.neck_fire_fold > 0.0);
    assert!(s.shoulder_fire_fold > s.hip_fire_fold);
    let neck_rest = {
        let mut r = SelfState::default_loadout();
        r.compose_joints();
        r.neck_fold
    };
    assert!(s.neck_fold > neck_rest);
}

#[test]
fn hip_and_neck_residual_land_on_joints_not_shoulder_only() {
    let mut s = SelfState::default_loadout();
    s.hip_fire_fold = 2f32.to_radians();
    s.neck_fire_fold = 3f32.to_radians();
    s.compose_joints();
    let (hip_tgt, shoulder_tgt) = elevation_targets(s.look_offset_pitch);
    assert!((s.hip_fold - (hip_tgt + s.hip_fire_fold)).abs() < 1e-6);
    assert!(s.neck_fold > hip_tgt);
    // Shoulder-only residual must not move hip/neck joints.
    let mut s2 = SelfState::default_loadout();
    s2.shoulder_fire_fold = 10f32.to_radians();
    s2.compose_joints();
    assert!((s2.hip_fold - hip_tgt).abs() < 1e-6);
    assert!((s2.shoulder_fold - (shoulder_tgt + s2.shoulder_fire_fold)).abs() < 1e-6);
    let mut rest = SelfState::default_loadout();
    rest.compose_joints();
    assert!((s2.neck_fold - rest.neck_fold).abs() < 1e-6);
}

#[test]
fn weapon_line_ignores_neck_residual() {
    let mut s = SelfState::default_loadout();
    s.neck_fire_fold = 8f32.to_radians();
    s.compose_joints();
    let wl = s.weapon_line().expect("armed");
    assert!(wl.dot(drive_look_dir(&s)) > 0.999);
    s.hip_fire_fold = 4f32.to_radians();
    s.compose_joints();
    let wl2 = s.weapon_line().expect("armed");
    assert!(wl2.y > wl.y);
}

#[test]
fn weapon_line_from_composed_joints_not_free_bag() {
    let mut s = SelfState::default_loadout();
    s.set_look(0.5, 0.2);
    s.hip_fire_fold = 3f32.to_radians();
    s.shoulder_fire_fold = 5f32.to_radians();
    s.shoulder_fire_twist = 2f32.to_radians();
    s.compose_joints();
    let wl = s.weapon_line().expect("armed");
    let yaw = s.facing + s.shoulder_twist;
    let pitch = s.hip_fold + s.shoulder_fold;
    let cp = pitch.cos();
    let expected = Vec3::new(yaw.sin() * cp, pitch.sin(), yaw.cos() * cp);
    assert!((wl - expected).length() < 1e-5);
    assert!((pitch - s.look_offset_pitch).abs() > 1e-3);
}

#[test]
fn wish_turns_with_look_ground_azimuth() {
    let mut s = SelfState::default_loadout();
    s.set_look(std::f32::consts::FRAC_PI_2, 0.0); // face +X
    s.apply_move(1.0, 1.0, 0.0, false);
    assert!((s.position.x - WALK_SPEED_M_S).abs() < 1e-4);
    assert!(s.position.z.abs() < 1e-4);
    assert!((s.facing - s.look_yaw()).abs() < 1e-6);
}

#[test]
fn walk_forward_along_look_at_constant_speed() {
    let mut s = SelfState::default_loadout();
    s.apply_move(1.0, 1.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Walk);
    assert!((s.position.z - WALK_SPEED_M_S).abs() < 1e-5);
    assert!(s.position.x.abs() < 1e-5);
    assert!(s.position.y.abs() < 1e-5);
    let expect_phase = (WALK_SPEED_M_S / WALK_STRIDE_M).rem_euclid(1.0);
    assert!((s.walk_phase - expect_phase).abs() < 1e-5);
}

#[test]
fn diagonal_wish_normalizes_speed() {
    let mut s = SelfState::default_loadout();
    s.apply_move(1.0, 1.0, 1.0, false);
    let dist = s.position.length();
    assert!((dist - WALK_SPEED_M_S).abs() < 1e-4, "dist={dist}");
}

#[test]
fn strafe_is_look_relative_and_keys_do_not_yaw() {
    let mut s = SelfState::default_loadout();
    s.facing = 0.0;
    s.look_offset_yaw = 0.0;
    s.sync_pose();
    s.apply_move(1.0, 0.0, 1.0, false);
    // Facing +Z, screen-right is −X (RH look_to / forward × up).
    assert!((s.position.x + WALK_SPEED_M_S).abs() < 1e-4);
    assert!(s.position.z.abs() < 1e-4);
    assert!((s.facing - s.look_yaw()).abs() < 1e-6);
    assert_eq!(s.facing, 0.0);
}

#[test]
fn zero_wish_settles_to_nearest_neutral_then_stands() {
    let mut s = SelfState::default_loadout();
    // First half: stop should aim at mid-cycle neutral (0.5), not full end.
    s.apply_move(0.1, 1.0, 0.0, false);
    assert!(
        s.walk_phase > 1e-6 && s.walk_phase < 0.5,
        "phase={}",
        s.walk_phase
    );
    let pos = s.position;

    s.apply_move(1e-3, 0.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Stopping);
    assert!((s.position - pos).length() < 1e-6, "feet plant on stop");

    let remain = 0.5 - s.walk_phase;
    let dt_finish = remain * WALK_STRIDE_M / WALK_SPEED_M_S + 1e-3;
    s.apply_move(dt_finish, 0.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Stand);
    assert!((s.walk_phase).abs() < 1e-6);
    assert!((s.position - pos).length() < 1e-6);
}

#[test]
fn stop_in_second_half_settles_to_cycle_end() {
    let mut s = SelfState::default_loadout();
    // One full stride-second lands past mid (speed/stride * t).
    s.apply_move(0.4, 1.0, 0.0, false);
    assert!(s.walk_phase >= 0.5, "phase={}", s.walk_phase);
    let pos = s.position;
    let remain = 1.0 - s.walk_phase;
    s.apply_move(1e-3, 0.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Stopping);
    let dt_finish = (remain - 1e-3 * WALK_SPEED_M_S / WALK_STRIDE_M).max(0.0) * WALK_STRIDE_M
        / WALK_SPEED_M_S
        + 1e-3;
    s.apply_move(dt_finish, 0.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Stand);
    assert!((s.position - pos).length() < 1e-6);
}

#[test]
fn walk_after_settled_stand_starts_at_phase_zero() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.1, 1.0, 0.0, false);
    let remain = 0.5 - s.walk_phase;
    let dt_finish = remain * WALK_STRIDE_M / WALK_SPEED_M_S + 1e-3;
    s.apply_move(dt_finish, 0.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Stand);

    s.apply_move(1e-3, 1.0, 0.0, false);
    let expect = (WALK_SPEED_M_S * 1e-3 / WALK_STRIDE_M).rem_euclid(1.0);
    assert!(
        (s.walk_phase - expect).abs() < 1e-5,
        "phase={}",
        s.walk_phase
    );
}

#[test]
fn wish_during_stopping_resumes_walk() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.15, 1.0, 0.0, false);
    s.apply_move(1e-3, 0.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Stopping);
    let phase = s.walk_phase;
    s.apply_move(1e-3, 1.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Walk);
    assert!(s.walk_phase > phase);
}

#[test]
fn jump_launches_to_air_and_peaks_near_target() {
    let mut s = SelfState::default_loadout();
    s.try_jump();
    assert_eq!(s.locomotion, LocomotionMode::Air);
    assert!((s.velocity_y - JUMP_LAUNCH_M_S).abs() < 1e-5);

    let dt = 1.0 / 120.0;
    let mut peak = 0.0_f32;
    for _ in 0..200 {
        s.apply_move(dt, 0.0, 0.0, false);
        peak = peak.max(s.position.y);
        if s.is_grounded() {
            break;
        }
    }
    assert!(
        (peak - JUMP_PEAK_M).abs() < 0.05,
        "peak={peak} want ~{JUMP_PEAK_M}"
    );
    assert_eq!(s.locomotion, LocomotionMode::Stand);
    assert!(s.position.y.abs() < 1e-5);
    assert!(s.velocity_y.abs() < 1e-5);
}

#[test]
fn jump_while_airborne_is_ignored() {
    let mut s = SelfState::default_loadout();
    s.try_jump();
    s.apply_move(0.05, 0.0, 0.0, false);
    let y = s.position.y;
    let vy = s.velocity_y;
    s.try_jump();
    assert!((s.position.y - y).abs() < 1e-6);
    assert!((s.velocity_y - vy).abs() < 1e-6);
    assert!(s.jump_buffer_s > 0.0);
}

#[test]
fn coyote_allows_jump_shortly_after_leaving_support() {
    use crate::{MapBox, MapWorld};
    let world = MapWorld {
        boxes: vec![MapBox {
            center: Vec3::new(0.0, 0.5, 0.0),
            half: Vec3::new(1.0, 0.5, 1.0),
            yaw: 0.0,
        }],
        ramps: vec![],
    };
    let mut s = SelfState::default_loadout();
    s.position = Vec3::new(0.0, 1.0, 0.0);
    let dt = 1.0 / 60.0;
    for _ in 0..20 {
        s.apply_move_world(dt, 0.0, 0.0, false, &world);
    }
    assert!(s.is_grounded());
    for _ in 0..40 {
        s.apply_move_world(dt, 1.0, 0.0, false, &world);
        if s.locomotion.is_air() {
            break;
        }
    }
    assert!(s.locomotion.is_air());
    assert!(
        s.coyote_s > 1e-6,
        "coyote should remain after leaving support"
    );
    s.try_jump();
    assert!(
        (s.velocity_y - JUMP_LAUNCH_M_S).abs() < 1e-5,
        "coyote jump should launch"
    );
    assert!(s.coyote_s.abs() < 1e-6);
}

#[test]
fn walk_off_edge_keeps_horizontal_air_speed() {
    use crate::{MapBox, MapWorld};
    let top = 1.12_f32;
    let world = MapWorld {
        boxes: vec![MapBox {
            center: Vec3::new(0.0, top * 0.5, 0.0),
            half: Vec3::new(2.0, top * 0.5, 1.0),
            yaw: 0.0,
        }],
        ramps: vec![],
    };
    let mut s = SelfState::default_loadout();
    s.position = Vec3::new(0.0, top, 0.0);
    let dt = 1.0 / 60.0;
    for _ in 0..20 {
        s.apply_move_world(dt, 0.0, 0.0, false, &world);
    }
    assert!(s.is_grounded());
    let mut left = false;
    for _ in 0..120 {
        s.apply_move_world(dt, 1.0, 0.0, false, &world);
        if s.locomotion.is_air() {
            left = true;
            break;
        }
    }
    assert!(left, "should walk off the +Z face");
    assert!(
        (s.air_vel_z - WALK_SPEED_M_S).abs() < 1e-3,
        "leave-edge should carry walk speed, air_z={}",
        s.air_vel_z
    );
    let z0 = s.position.z;
    // A few air frames while still beside the tall box face.
    let mut moved = 0.0_f32;
    for _ in 0..10 {
        let z_before = s.position.z;
        s.apply_move_world(dt, 1.0, 0.0, false, &world);
        moved += s.position.z - z_before;
        assert!(s.locomotion.is_air());
    }
    assert!(
        moved > WALK_SPEED_M_S * dt * 5.0,
        "expected coast away from ledge, moved={moved} from z0={z0} to z={}",
        s.position.z
    );
}

#[test]
fn jump_buffer_fires_on_land() {
    let mut s = SelfState::default_loadout();
    s.try_jump();
    let dt = 1.0 / 120.0;
    // Fall until near the ground with coyote already expired.
    for _ in 0..400 {
        s.apply_move(dt, 0.0, 0.0, false);
        if s.position.y < 0.4 && s.velocity_y < 0.0 && s.coyote_s <= 1e-6 {
            break;
        }
    }
    assert!(s.locomotion.is_air());
    assert!(s.coyote_s <= 1e-6);
    s.try_jump();
    assert!(s.jump_buffer_s > 0.0);

    for _ in 0..60 {
        s.apply_move(dt, 0.0, 0.0, false);
        if (s.velocity_y - JUMP_LAUNCH_M_S).abs() < 1e-4 {
            assert!(
                s.position.y < 0.35,
                "buffered jump should relaunch near the ground, y={}",
                s.position.y
            );
            return;
        }
    }
    panic!("expected buffered jump on land");
}

#[test]
fn air_coasts_at_launch_direction_and_freezes_phase() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.1, 1.0, 0.0, false);
    let phase = s.walk_phase;
    s.try_jump();
    // Strafe mid-air must not change path — still +Z from launch.
    s.apply_move(0.2, 0.0, 1.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Air);
    assert!((s.walk_phase - phase).abs() < 1e-6, "phase must freeze");
    assert!(
        (s.position.z - WALK_SPEED_M_S * 0.3).abs() < 1e-3,
        "z={}",
        s.position.z
    );
    assert!(s.position.x.abs() < 1e-3, "x={}", s.position.x);
}

#[test]
fn land_with_wish_enters_walk() {
    let mut s = SelfState::default_loadout();
    s.try_jump();
    let dt = 1.0 / 60.0;
    for _ in 0..120 {
        s.apply_move(dt, 1.0, 0.0, false);
        if !s.locomotion.is_air() {
            break;
        }
    }
    assert_eq!(s.locomotion, LocomotionMode::Walk);
    assert!(s.position.y.abs() < 1e-5);
    assert!((s.walk_phase).abs() < 1e-6);
}

#[test]
fn sprint_moves_faster_than_walk() {
    let mut walk = SelfState::default_loadout();
    let mut sprint = SelfState::default_loadout();
    walk.apply_move(1.0, 1.0, 0.0, false);
    sprint.apply_move(1.0, 1.0, 0.0, true);
    assert_eq!(sprint.locomotion, LocomotionMode::Sprint);
    assert!(sprint.sprint_latched);
    assert!((walk.position.z - WALK_SPEED_M_S).abs() < 1e-4);
    assert!((sprint.position.z - SPRINT_SPEED_M_S).abs() < 1e-4);
    assert!(sprint.stamina < STAMINA_MAX);
}

#[test]
fn sprint_stays_without_holding() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.05, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
    s.apply_move(0.2, 1.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
    assert!(s.sprint_latched);
}

#[test]
fn second_tap_keeps_sprint() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.05, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
    s.apply_move(0.05, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
    assert!(s.sprint_latched);
}

#[test]
fn sprint_requires_min_stamina_to_start() {
    let mut s = SelfState::default_loadout();
    s.stamina = STAMINA_MIN_TO_START - 0.01;
    s.apply_move(0.1, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Walk);
    assert!(!s.sprint_latched);
    assert!((s.position.z - WALK_SPEED_M_S * 0.1).abs() < 1e-4);
}

#[test]
fn sprint_continues_below_min_until_empty() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.05, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
    s.stamina = STAMINA_MIN_TO_START - 0.05;
    s.apply_move(0.05, 1.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
}

#[test]
fn empty_stamina_drops_to_walk_without_restart() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.05, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
    s.stamina = 1e-4;
    s.apply_move(0.05, 1.0, 0.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Walk);
    assert!(!s.sprint_latched);
    assert!(s.stamina < STAMINA_MIN_TO_START);
    let z = s.position.z;
    // Fresh tap still blocked until min fill.
    s.apply_move(0.1, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Walk);
    assert!((s.position.z - z - WALK_SPEED_M_S * 0.1).abs() < 1e-3);
}

#[test]
fn stamina_regens_when_not_sprinting() {
    let mut s = SelfState::default_loadout();
    s.stamina = 0.0;
    s.apply_move(STAMINA_REGEN_S, 0.0, 0.0, false);
    assert!((s.stamina - STAMINA_MAX).abs() < 1e-4);
}

#[test]
fn stop_wish_clears_sprint_latch() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.1, 1.0, 0.0, true);
    assert!(s.sprint_latched);
    s.apply_move(0.1, 0.0, 0.0, false);
    assert!(!s.sprint_latched);
    assert_ne!(s.locomotion, LocomotionMode::Sprint);
}

#[test]
fn air_does_not_start_sprint() {
    let mut s = SelfState::default_loadout();
    s.try_jump();
    s.apply_move(0.05, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Air);
}

#[test]
fn jump_from_sprint_locks_sprint_air_speed() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.1, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
    s.try_jump();
    assert!((s.air_vel_z - SPRINT_SPEED_M_S).abs() < 1e-4);
    assert!(s.air_vel_x.abs() < 1e-5);
}

#[test]
fn sprint_rejects_strafe_only_and_back() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.1, 0.0, 1.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Walk);
    assert!(!s.sprint_latched);
    assert!((s.position.length() - WALK_SPEED_M_S * 0.1).abs() < 1e-3);

    s = SelfState::default_loadout();
    s.apply_move(0.1, -1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Walk);
    assert!(!s.sprint_latched);
}

#[test]
fn sprint_allows_forward_with_strafe() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.1, 1.0, 1.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
    assert!((s.position.length() - SPRINT_SPEED_M_S * 0.1).abs() < 1e-3);
}

#[test]
fn losing_forward_ends_sprint() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.05, 1.0, 0.0, true);
    assert_eq!(s.locomotion, LocomotionMode::Sprint);
    s.apply_move(0.05, 0.0, 1.0, false);
    assert_eq!(s.locomotion, LocomotionMode::Walk);
    assert!(!s.sprint_latched);
}

#[test]
fn latched_sprint_drains_in_air_no_jump_regen() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.05, 1.0, 0.0, true);
    assert!(s.sprint_latched);
    let before = s.stamina;
    s.try_jump();
    assert_eq!(s.locomotion, LocomotionMode::Air);
    // Full hop time is short; still must drain, never regen.
    let dt = 1.0 / 60.0;
    for _ in 0..30 {
        s.apply_move(dt, 1.0, 0.0, false);
    }
    assert!(
        s.stamina < before - 0.05,
        "stamina={before} -> {} (expected drain while latched aloft)",
        s.stamina
    );
    assert!(s.stamina < before);
}

#[test]
fn weapon_class_map_covers_a_through_r() {
    assert_eq!(WeaponClass::from_letter(b'a'), Some(WeaponClass::Launcher));
    assert_eq!(WeaponClass::from_letter(b'b'), Some(WeaponClass::Pistol));
    assert_eq!(WeaponClass::from_letter(b'p'), Some(WeaponClass::Smg));
    assert_eq!(WeaponClass::from_letter(b'z'), None);
    assert!(WeaponClass::Launcher.allowed_in_secondary());
    assert!(WeaponClass::Pistol.allowed_in_secondary());
    assert!(!WeaponClass::Smg.allowed_in_secondary());
}

#[test]
fn secondary_rejects_non_sidearm() {
    let mut s = SelfState::default_loadout();
    assert!(s.set_secondary(Some(b'p')).is_err());
    assert_eq!(s.secondary, Some(b'b'));
    assert!(s.set_secondary(Some(b'i')).is_ok());
    assert_eq!(s.secondary, Some(b'i'));
    assert!(s.set_secondary(None).is_ok());
    assert_eq!(s.secondary, None);
}

#[test]
fn primary_accepts_any_class() {
    let mut s = SelfState::default_loadout();
    assert!(s.set_primary(Some(b'a')).is_ok());
    assert_eq!(s.primary, Some(b'a'));
    assert!(s.set_primary(Some(b'e')).is_ok());
    assert!(s.set_primary(None).is_ok());
    assert_eq!(s.primary, None);
    // Still on primary slot — empty means unarmed, not a third mode.
    assert_eq!(s.active, ActiveWeapon::Primary);
    assert!(!s.is_armed());
}

#[test]
fn cycle_weapon_toggles_two_slots_only() {
    let mut s = SelfState::default_loadout();
    assert_eq!(s.active, ActiveWeapon::Primary);
    s.cycle_weapon(1);
    assert_eq!(s.active, ActiveWeapon::Secondary);
    assert_eq!(s.active_blaster(), Some(b'b'));
    s.cycle_weapon(1);
    assert_eq!(s.active, ActiveWeapon::Primary);
    assert_eq!(s.active_blaster(), Some(b'p'));
    // Both filled → always armed; no free third unarmed step.
    assert!(s.is_armed());

    s.set_secondary(None).unwrap();
    s.active = ActiveWeapon::Primary;
    // Empty secondary is skipped while primary is equipped (081).
    s.cycle_weapon(1);
    assert_eq!(s.active, ActiveWeapon::Primary);
    assert!(s.is_armed());
    assert!(s.weapon_line().is_some());
}

#[test]
fn coerce_active_armed_prefers_filled_slot() {
    let mut s = SelfState::default_loadout();
    s.set_primary(None).unwrap();
    s.active = ActiveWeapon::Primary;
    assert!(!s.is_armed());
    s.coerce_active_armed();
    assert_eq!(s.active, ActiveWeapon::Secondary);
    assert_eq!(s.active_blaster(), Some(b'b'));

    assert_eq!(
        prefer_armed_slot(None, Some(b'b'), ActiveWeapon::Primary),
        ActiveWeapon::Secondary
    );
    assert_eq!(
        prefer_armed_slot(Some(b'p'), None, ActiveWeapon::Secondary),
        ActiveWeapon::Primary
    );
    assert_eq!(
        prefer_armed_slot(None, None, ActiveWeapon::Primary),
        ActiveWeapon::Primary
    );
}

#[test]
fn continuous_jump_while_latched_empties_stamina() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.02, 1.0, 0.0, true);
    let dt = 1.0 / 60.0;
    for _ in 0..500 {
        if s.is_grounded() {
            s.try_jump();
        }
        s.apply_move(dt, 1.0, 0.0, false);
        if !s.sprint_latched && s.stamina <= 0.0 {
            break;
        }
    }
    assert!(
        !s.sprint_latched && s.stamina <= 1e-5,
        "latch={} stamina={}",
        s.sprint_latched,
        s.stamina
    );
}

#[test]
fn emote_commit_requires_ground_and_clears_sprint() {
    let mut s = SelfState::default_loadout();
    s.apply_move(0.05, 1.0, 0.0, true);
    assert!(s.sprint_latched);
    assert!(s.try_commit_emote(0, false));
    assert_eq!(s.emote, Some(0));
    assert!(!s.sprint_latched);
    assert!(s.emote_holster());
    assert!(!s.presents_armed());
    assert!(s.is_armed());
}

#[test]
fn emote_blocked_in_air_and_by_weapon_side() {
    let mut s = SelfState::default_loadout();
    s.try_jump();
    assert!(!s.try_commit_emote(0, false));
    s = SelfState::default_loadout();
    assert!(!s.try_commit_emote(0, true));
}

#[test]
fn emote_ends_after_duration_and_move_cancels() {
    let mut s = SelfState::default_loadout();
    assert!(s.try_commit_emote(3, false)); // bow 0.33s
    s.tick_emote(0.2);
    assert!(s.is_emoting());
    s.tick_emote(0.2);
    assert!(!s.is_emoting());

    assert!(s.try_commit_emote(0, false));
    s.apply_move(0.01, 1.0, 0.0, false);
    assert!(!s.is_emoting());
}

#[test]
fn jump_and_cycle_cancel_emote() {
    let mut s = SelfState::default_loadout();
    assert!(s.try_commit_emote(1, false));
    s.try_jump();
    assert!(!s.is_emoting());

    s = SelfState::default_loadout();
    assert!(s.try_commit_emote(2, false));
    s.cycle_weapon(1);
    assert!(!s.is_emoting());
}

#[test]
fn tiny_mouse_yaw_moves_facing() {
    let mut s = SelfState::default_loadout();
    let sens = 0.00015_f32;
    let d = -3.0 * sens; // 3 px horizontal
    let before = s.facing;
    s.apply_look(1.0 / 60.0, d, 0.0);
    assert!(
        (s.facing - (before + d)).abs() < 1e-9,
        "facing {} before {} d {}",
        s.facing,
        before,
        d
    );
    let deg = (s.facing - before).to_degrees().abs();
    assert!(deg > 0.01, "expected >0.01 deg, got {deg}");
}

#[test]
fn step_up_onto_low_box_without_jump() {
    use crate::{MapBox, MapWorld, STEP_UP_M};
    let top = STEP_UP_M * 0.8;
    let world = MapWorld {
        boxes: vec![MapBox {
            center: Vec3::new(0.0, top * 0.5, 2.0),
            half: Vec3::new(1.0, top * 0.5, 1.0),
            yaw: 0.0,
        }],
        ramps: vec![],
    };
    let mut s = SelfState::default_loadout();
    let mut peaked = 0.0_f32;
    for _ in 0..120 {
        s.apply_move_world(1.0 / 60.0, 1.0, 0.0, false, &world);
        peaked = peaked.max(s.position.y);
        if s.is_grounded() && s.position.y > top * 0.5 {
            break;
        }
    }
    assert!(s.is_grounded());
    assert!(
        (s.position.y - top).abs() < 0.05,
        "y={} peaked={peaked} want ~{top}",
        s.position.y
    );
}

#[test]
fn tall_box_blocks_without_jump() {
    use crate::{MapBox, MapWorld};
    let world = MapWorld {
        boxes: vec![MapBox {
            center: Vec3::new(0.0, 0.6, 2.0),
            half: Vec3::new(1.0, 0.6, 1.0),
            yaw: 0.0,
        }],
        ramps: vec![],
    };
    let mut s = SelfState::default_loadout();
    for _ in 0..180 {
        s.apply_move_world(1.0 / 60.0, 1.0, 0.0, false, &world);
    }
    assert!(s.position.y.abs() < 0.05, "y={}", s.position.y);
    assert!(
        s.position.z < 1.2,
        "should not walk through tall box, z={}",
        s.position.z
    );
}

#[test]
fn jump_onto_mid_box_and_land_grounded() {
    use crate::{MapBox, MapWorld};
    let top = 0.9_f32;
    let world = MapWorld {
        boxes: vec![MapBox {
            center: Vec3::new(0.0, top * 0.5, 2.0),
            half: Vec3::new(1.5, top * 0.5, 1.5),
            yaw: 0.0,
        }],
        ramps: vec![],
    };
    let mut s = SelfState::default_loadout();
    // Walk up to the face, jump, coast onto the top.
    for _ in 0..40 {
        s.apply_move_world(1.0 / 60.0, 1.0, 0.0, false, &world);
    }
    s.try_jump();
    for _ in 0..180 {
        s.apply_move_world(1.0 / 60.0, 1.0, 0.0, false, &world);
        if s.is_grounded() && s.position.y > top * 0.5 {
            break;
        }
    }
    assert!(s.is_grounded());
    assert!(
        (s.position.y - top).abs() < 0.08,
        "y={} want ~{top}",
        s.position.y
    );
}

#[test]
fn walk_up_ramp_to_height() {
    use crate::{MapRamp, MapWorld};
    let world = MapWorld {
        boxes: vec![],
        ramps: vec![MapRamp {
            center_x: 0.0,
            center_z: 2.0,
            half_x: 1.0,
            half_z: 2.0,
            height: 1.0,
            base_y: 0.0,
            yaw: 0.0,
        }],
    };
    let mut s = SelfState::default_loadout();
    let mut peaked = 0.0_f32;
    for _ in 0..200 {
        s.apply_move_world(1.0 / 60.0, 1.0, 0.0, false, &world);
        peaked = peaked.max(s.position.y);
        if s.is_grounded() && s.position.y > 0.7 {
            break;
        }
    }
    assert!(s.is_grounded());
    assert!(
        s.position.y > 0.7,
        "expected ramp height, y={} peaked={peaked}",
        s.position.y
    );
}
