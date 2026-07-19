//! Map net poses onto local `SelfState` (presentation drive under authority).

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

pub fn apply_pose(state: &mut SelfState, pose: &NetPlayerPose) {
    state.position = Vec3::new(pose.position.x, pose.position.y, pose.position.z);
    state.set_look(pose.ocular_yaw, pose.ocular_pitch);
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
}
