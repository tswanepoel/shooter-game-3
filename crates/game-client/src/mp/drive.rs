use game_net::{DriveView, NetActiveWeapon, NetLocomotion, NetVec3};
use game_sim::{ActiveWeapon, LocomotionMode, SelfState};
use glam::Vec3;

pub fn state_to_drive(state: &SelfState) -> DriveView {
    DriveView {
        position: NetVec3::new(state.position.x, state.position.y, state.position.z),
        ocular_yaw: state.ocular_yaw,
        ocular_pitch: state.ocular_pitch,
        character: state.character,
        primary: state.primary,
        secondary: state.secondary,
        active: match state.active {
            ActiveWeapon::Primary => NetActiveWeapon::Primary,
            ActiveWeapon::Secondary => NetActiveWeapon::Secondary,
        },
        locomotion: match state.locomotion {
            LocomotionMode::Stand => NetLocomotion::Stand,
            LocomotionMode::Walk => NetLocomotion::Walk,
            LocomotionMode::Sprint => NetLocomotion::Sprint,
            LocomotionMode::Stopping => NetLocomotion::Stopping,
            LocomotionMode::Air => NetLocomotion::Air,
        },
        walk_phase: state.walk_phase,
        velocity_y: state.velocity_y,
        emote: state.emote,
        emote_age_s: state.emote_age_s,
    }
}

pub fn drive_to_state(drive: &DriveView) -> SelfState {
    let mut state = SelfState::default_loadout();
    apply_drive(&mut state, drive);
    state
}

pub fn apply_drive(state: &mut SelfState, drive: &DriveView) {
    state.position = Vec3::new(drive.position.x, drive.position.y, drive.position.z);
    state.character = drive.character;
    state.primary = drive.primary;
    state.secondary = drive.secondary;
    state.active = match drive.active {
        NetActiveWeapon::Primary => ActiveWeapon::Primary,
        NetActiveWeapon::Secondary => ActiveWeapon::Secondary,
    };
    state.locomotion = match drive.locomotion {
        NetLocomotion::Stand => LocomotionMode::Stand,
        NetLocomotion::Walk => LocomotionMode::Walk,
        NetLocomotion::Sprint => LocomotionMode::Sprint,
        NetLocomotion::Stopping => LocomotionMode::Stopping,
        NetLocomotion::Air => LocomotionMode::Air,
    };
    state.walk_phase = drive.walk_phase;
    state.velocity_y = drive.velocity_y;
    state.sprint_latched = matches!(drive.locomotion, NetLocomotion::Sprint);
    state.emote = drive.emote;
    state.emote_age_s = drive.emote_age_s;
    state.set_look(drive.ocular_yaw, drive.ocular_pitch);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_present_fields() {
        let mut s = SelfState::default_loadout();
        s.position = Vec3::new(3.0, 0.5, -1.0);
        s.set_look(1.2, -0.3);
        s.locomotion = LocomotionMode::Sprint;
        s.walk_phase = 0.4;
        s.character = b'c';
        s.primary = Some(b'd');
        s.secondary = Some(b'a');
        s.active = ActiveWeapon::Secondary;

        let d = state_to_drive(&s);
        let back = drive_to_state(&d);
        assert!((back.position.x - 3.0).abs() < 1e-5);
        assert!((back.position.y - 0.5).abs() < 1e-5);
        assert!((back.ocular_yaw - 1.2).abs() < 1e-5);
        assert!((back.ocular_pitch - (-0.3)).abs() < 1e-5);
        assert_eq!(back.locomotion, LocomotionMode::Sprint);
        assert!((back.walk_phase - 0.4).abs() < 1e-5);
        assert_eq!(back.character, b'c');
        assert_eq!(back.primary, Some(b'd'));
        assert_eq!(back.secondary, Some(b'a'));
        assert_eq!(back.active, ActiveWeapon::Secondary);
        assert!(back.sprint_latched);
        assert_eq!(back.emote, None);
        assert_eq!(back.emote_age_s, 0.0);

        s.emote = Some(2);
        s.emote_age_s = 0.2;
        let d = state_to_drive(&s);
        let back = drive_to_state(&d);
        assert_eq!(back.emote, Some(2));
        assert!((back.emote_age_s - 0.2).abs() < 1e-5);
    }
}
