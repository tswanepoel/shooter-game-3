//! Ground-truth world quantities shared by sim and client.
//!
//! World space: 1 unit = 1 metre, Y-up, XZ ground plane.
//! Values are pure data — not behaviour. Prefer naming the real-world driver.

/// Standing adult eye height (metres). Placeholder until the camera mounts on a body.
pub const STANDING_EYE_HEIGHT_M: f32 = 1.7;

/// Vertical field of view for the temporary fixed spectator camera (radians).
///
/// Not a physical constant of the world; a deliberate projection choice (~75°).
pub const CAMERA_VERTICAL_FOV_RAD: f32 = 75.0_f32.to_radians();

/// Near clip plane (metres). Closer than typical hand/weapon scale would clip.
pub const CAMERA_NEAR_M: f32 = 0.1;

/// Far clip plane (metres). Beyond typical outdoor engagement / debug grid extent.
pub const CAMERA_FAR_M: f32 = 500.0;

/// Debug grid minor line spacing (metres). Human-scale cell size.
pub const GRID_MINOR_SPACING_M: f32 = 1.0;

/// Draw a major grid line every N minor cells (→ 10 m with default minor spacing).
pub const GRID_MAJOR_EVERY: u32 = 10;

/// Half-extent of the debug grid along X and Z (metres). Draw extent only, not a world bound.
pub const DEBUG_GRID_HALF_EXTENT_M: f32 = 50.0;

const _: () = {
    assert!(STANDING_EYE_HEIGHT_M > 1.0 && STANDING_EYE_HEIGHT_M < 2.5);
    assert!(CAMERA_NEAR_M > 0.0);
    assert!(CAMERA_FAR_M > CAMERA_NEAR_M);
    assert!(GRID_MINOR_SPACING_M > 0.0);
    assert!(GRID_MAJOR_EVERY > 0);
    assert!(DEBUG_GRID_HALF_EXTENT_M >= GRID_MINOR_SPACING_M);
};
