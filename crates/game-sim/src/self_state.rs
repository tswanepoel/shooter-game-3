//! Player self: placement, facing, kit identity, eye mount.

use glam::{Mat4, Quat, Vec3};

/// Local eye offset from feet (metres).
pub const DEFAULT_SELF_EYE_OFFSET_M: Vec3 = Vec3::new(0.0, 1.52, 0.27);

#[derive(Debug, Clone, PartialEq)]
pub struct SelfState {
    /// Feet position (world metres).
    pub position: Vec3,
    /// Yaw about world Y (radians). 0 faces **+Z**.
    pub yaw: f32,
    /// Character kit letter `'a'`..=`'r'`.
    pub character: u8,
    /// Blaster kit letter `'a'`..=`'r'`.
    pub blaster: u8,
    /// Local eye offset from feet (metres); rotates with yaw.
    pub eye_offset: Vec3,
}

impl Default for SelfState {
    fn default() -> Self {
        Self::default_loadout()
    }
}

impl SelfState {
    pub fn default_loadout() -> Self {
        Self {
            position: Vec3::ZERO,
            yaw: 0.0,
            character: b'a',
            blaster: b'p',
            eye_offset: DEFAULT_SELF_EYE_OFFSET_M,
        }
    }

    /// Unit facing on XZ (identity yaw → +Z).
    pub fn facing(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos())
    }

    pub fn eye_world(&self) -> Vec3 {
        let q = Quat::from_rotation_y(self.yaw);
        self.position + q * self.eye_offset
    }

    pub fn placement_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_y(self.yaw), self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_faces_plus_z_at_origin() {
        let s = SelfState::default_loadout();
        assert_eq!(s.position, Vec3::ZERO);
        assert_eq!(s.character, b'a');
        assert_eq!(s.blaster, b'p');
        let f = s.facing();
        assert!(f.dot(Vec3::Z) > 0.99);
        assert!((f.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn eye_is_local_offset_rotated_by_yaw() {
        let mut s = SelfState::default_loadout();
        s.eye_offset = Vec3::new(0.0, 1.5, 0.1);
        let eye0 = s.eye_world();
        assert!((eye0 - Vec3::new(0.0, 1.5, 0.1)).length() < 1e-5);

        s.yaw = std::f32::consts::FRAC_PI_2;
        let eye1 = s.eye_world();
        // +Z local → +X after +90° yaw.
        assert!((eye1 - Vec3::new(0.1, 1.5, 0.0)).length() < 1e-4);
        assert!(s.facing().dot(Vec3::X) > 0.99);
    }
}
