//! Map net poses onto local `SelfState` (present drive and authority body).

use game_net::{NetActiveWeapon, NetLocomotion, NetPlayerPose, NetSpawn};
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

/// Authoritative body only — does not touch ocular yaw/pitch (local camera).
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
    // Sprint latch is not on the wire. Infer from authority loco.
    state.sprint_latched = match pose.locomotion {
        NetLocomotion::Sprint => true,
        NetLocomotion::Air => state.sprint_latched,
        _ => false,
    };
    if state.sprint_latched && state.stamina <= 0.0 {
        state.stamina = f32::EPSILON;
    }
    state.sync_pose();
}

/// One sent Input held until land time (032); body sim uses held channels at tick rate.
#[derive(Debug, Clone)]
pub struct LandSample {
    pub input: game_net::Input,
    /// Client clock when this sample may enter the held body channels.
    pub land_at_secs: f64,
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
    fn sprint_pose_restores_latch() {
        let mut state = SelfState::default_loadout();
        state.sprint_latched = false;
        state.stamina = 0.0;
        let mut pose = pose_at(0.0, 0.0);
        pose.locomotion = NetLocomotion::Sprint;
        apply_body_from_pose(&mut state, &pose);
        assert!(state.sprint_latched);
        assert!(state.stamina > 0.0);
        assert_eq!(state.locomotion, LocomotionMode::Sprint);
    }
}
