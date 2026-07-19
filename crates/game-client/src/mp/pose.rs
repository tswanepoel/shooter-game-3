//! Map net poses onto local `SelfState` (present drive and predict/reconcile).

use game_net::{Input, NetActiveWeapon, NetLocomotion, NetPlayerPose, NetSpawn};
use game_sim::{ActiveWeapon, LocomotionMode, SelfState};
use glam::Vec3;

pub fn apply_spawn(state: &mut SelfState, spawn: &NetSpawn) {
    state.position = Vec3::new(spawn.position.x, spawn.position.y, spawn.position.z);
    state.set_look(spawn.yaw, 0.0);
    state.wish_forward = 0.0;
    state.wish_strafe = 0.0;
    state.locomotion = LocomotionMode::Stand;
    state.walk_phase = 0.0;
    state.velocity_y = 0.0;
    state.air_vel_x = 0.0;
    state.air_vel_z = 0.0;
}

/// Full pose including look (remotes / spawn-style present).
pub fn apply_pose(state: &mut SelfState, pose: &NetPlayerPose) {
    apply_body_from_pose(state, pose);
    state.set_look(pose.ocular_yaw, pose.ocular_pitch);
}

/// Authoritative body only — does not touch ocular yaw/pitch (026 local camera).
pub fn apply_body_from_pose(state: &mut SelfState, pose: &NetPlayerPose) {
    state.position = Vec3::new(pose.position.x, pose.position.y, pose.position.z);
    state.character = pose.character;
    state.primary = pose.primary;
    state.secondary = pose.secondary;
    state.active = match pose.active {
        NetActiveWeapon::Primary => ActiveWeapon::Primary,
        NetActiveWeapon::Secondary => ActiveWeapon::Secondary,
    };
    state.locomotion = match pose.locomotion {
        NetLocomotion::Stand => LocomotionMode::Stand,
        NetLocomotion::Walk => LocomotionMode::Walk,
        NetLocomotion::Sprint => LocomotionMode::Sprint,
        NetLocomotion::Stopping => LocomotionMode::Stopping,
        NetLocomotion::Air => LocomotionMode::Air,
    };
    state.walk_phase = pose.walk_phase;
    state.velocity_y = pose.velocity_y;
    // air_vel / stamina / sprint latch are not on the wire; hard-correct may
    // mispredict mid-air until the next grounded sample.
    // Refresh body presentation from the preserved local look.
    state.sync_pose();
}

/// Apply one Input the way the server tick does (look → jump → weapon → move).
pub fn apply_input_to_state(state: &mut SelfState, input: &Input, dt: f32) {
    state.set_look(input.look_yaw, input.look_pitch);
    if input.jump {
        state.try_jump();
    }
    if input.weapon_cycle != 0 {
        state.cycle_weapon(input.weapon_cycle);
    }
    state.apply_move(dt, input.wish_forward, input.wish_strafe, input.sprint_tap);
}

/// Predict from a local intent (look already applied by the frame loop).
pub fn predict_intent(state: &mut SelfState, intent: &super::InputIntent, dt: f32) {
    if intent.jump {
        state.try_jump();
    }
    if intent.weapon_cycle != 0 {
        state.cycle_weapon(intent.weapon_cycle);
    }
    state.apply_move(
        dt,
        intent.wish_forward,
        intent.wish_strafe,
        intent.sprint_tap,
    );
}

/// Hard-set body from `you`, drop acked history, resim the rest; restore camera look.
pub fn reconcile_predicted(
    state: &mut SelfState,
    you: &NetPlayerPose,
    ack_seq: u32,
    history: &mut std::collections::VecDeque<PredictedSample>,
) {
    let yaw = state.ocular_yaw;
    let pitch = state.ocular_pitch;

    apply_body_from_pose(state, you);
    history.retain(|s| s.input.seq > ack_seq);
    for sample in history.iter() {
        apply_input_to_state(state, &sample.input, sample.dt);
    }
    state.set_look(yaw, pitch);
}

/// One sent Input plus the dt used when it was predicted.
#[derive(Debug, Clone)]
pub struct PredictedSample {
    pub input: Input,
    pub dt: f32,
}

/// Build a presentation drive from a net pose (remotes).
pub fn pose_to_state(pose: &NetPlayerPose) -> SelfState {
    let mut state = SelfState::default_loadout();
    apply_pose(&mut state, pose);
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_net::{NetActiveWeapon, NetLocomotion, NetVec3};

    fn pose_at(x: f32, z: f32) -> NetPlayerPose {
        NetPlayerPose {
            id: 1,
            position: NetVec3::new(x, 0.0, z),
            ocular_yaw: 1.5,
            ocular_pitch: 0.25,
            character: b'a',
            primary: Some(b'p'),
            secondary: Some(b'b'),
            active: NetActiveWeapon::Primary,
            locomotion: NetLocomotion::Stand,
            walk_phase: 0.0,
            velocity_y: 0.0,
        }
    }

    #[test]
    fn body_apply_leaves_look() {
        let mut state = SelfState::default_loadout();
        state.set_look(0.1, 0.2);
        apply_body_from_pose(&mut state, &pose_at(3.0, 4.0));
        assert!((state.position.x - 3.0).abs() < 1e-5);
        assert!((state.ocular_yaw - 0.1).abs() < 1e-5);
        assert!((state.ocular_pitch - 0.2).abs() < 1e-5);
    }

    #[test]
    fn reconcile_drops_acked_and_restores_look() {
        let mut state = SelfState::default_loadout();
        state.set_look(0.3, -0.1);
        state.position = Vec3::new(10.0, 0.0, 10.0);

        let mut history = std::collections::VecDeque::new();
        history.push_back(PredictedSample {
            input: Input {
                seq: 1,
                echo_key: 0,
                echo_issued_tick: 0,
                wish_forward: 1.0,
                wish_strafe: 0.0,
                look_yaw: 0.0,
                look_pitch: 0.0,
                jump: false,
                sprint_tap: false,
                weapon_cycle: 0,
            },
            dt: 1.0 / 60.0,
        });
        history.push_back(PredictedSample {
            input: Input {
                seq: 2,
                echo_key: 0,
                echo_issued_tick: 0,
                wish_forward: 1.0,
                wish_strafe: 0.0,
                look_yaw: 0.0,
                look_pitch: 0.0,
                jump: false,
                sprint_tap: false,
                weapon_cycle: 0,
            },
            dt: 1.0 / 60.0,
        });

        reconcile_predicted(&mut state, &pose_at(0.0, 0.0), 1, &mut history);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].input.seq, 2);
        assert!((state.ocular_yaw - 0.3).abs() < 1e-5);
        assert!((state.ocular_pitch - (-0.1)).abs() < 1e-5);
        // Resim of seq 2 moved forward from origin.
        assert!(state.position.z > 0.0);
    }
}
