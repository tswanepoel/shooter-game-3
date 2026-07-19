//! Map sim drive ↔ net pose DTOs (boundary only).

use game_net::{NetActiveWeapon, NetLocomotion, NetPlayerPose, NetVec3, PlayerId};
use game_sim::{ActiveWeapon, LocomotionMode, SelfState};

pub fn net_vec3(v: glam::Vec3) -> NetVec3 {
    NetVec3::new(v.x, v.y, v.z)
}

pub fn locomotion(m: LocomotionMode) -> NetLocomotion {
    match m {
        LocomotionMode::Stand => NetLocomotion::Stand,
        LocomotionMode::Walk => NetLocomotion::Walk,
        LocomotionMode::Sprint => NetLocomotion::Sprint,
        LocomotionMode::Stopping => NetLocomotion::Stopping,
        LocomotionMode::Air => NetLocomotion::Air,
    }
}

pub fn active_weapon(a: ActiveWeapon) -> NetActiveWeapon {
    match a {
        ActiveWeapon::Primary => NetActiveWeapon::Primary,
        ActiveWeapon::Secondary => NetActiveWeapon::Secondary,
    }
}

pub fn player_pose(id: PlayerId, state: &SelfState) -> NetPlayerPose {
    NetPlayerPose {
        id,
        position: net_vec3(state.position),
        ocular_yaw: state.ocular_yaw,
        ocular_pitch: state.ocular_pitch,
        character: state.character,
        primary: state.primary,
        secondary: state.secondary,
        active: active_weapon(state.active),
        locomotion: locomotion(state.locomotion),
        walk_phase: state.walk_phase,
        velocity_y: state.velocity_y,
    }
}
