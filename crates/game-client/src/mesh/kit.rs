use glam::{Mat4, Quat, Vec3};
use wasm_bindgen::JsValue;

use crate::pack::{self, Pack};

pub const KENNEY_CORE_PACK: &str = "kenney-core";

/// Character kit units → metres (2.7 kit → 1.8 m).
pub const CHAR_UNITS_TO_M: f32 = 1.0 / 1.5;
/// Blaster kit units → metres (1:1).
pub const BLASTER_UNITS_TO_M: f32 = 1.0;
/// Relative blaster scale when positions already ride the character scale chain.
pub const BLASTER_RELATIVE_SCALE: f32 = BLASTER_UNITS_TO_M / CHAR_UNITS_TO_M;

/// `holding-right` on `arm-right` (−90° X).
pub(crate) const HOLDING_RIGHT_ROT: Quat = Quat::from_xyzw(
    std::f32::consts::FRAC_1_SQRT_2,
    0.0,
    0.0,
    -std::f32::consts::FRAC_1_SQRT_2,
);

/// Hand socket **H_hold** under armed hold / aim (arm-local).
///
/// Orientation only: cancel `holding-right` (−90° X → +90° X) then yaw 180° so mesh
/// muzzle (−Z) faces character +Z with top +Y under hold. Fist placement is carried
/// by per-letter grip **G** (mesh origin on the socket). See feature 037.
#[inline]
pub fn hand_socket_hold() -> Mat4 {
    Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2) * Mat4::from_rotation_y(std::f32::consts::PI)
}

/// Weapon grip **G** translation in blaster-local units (handle / mesh origin relative
/// to the hand socket). Migrated from former arm-attachment grip offsets so
/// `H · inv(G)` matches the hold baseline. See blaster kit README.
const BLASTER_GRIP_G: [[f32; 3]; 18] = [
    [0.0, -0.34, 1.14], // a
    [0.0, -0.30, 1.00], // b
    [0.0, -0.20, 1.11], // c
    [0.0, -0.18, 1.11], // d
    [0.0, -0.22, 2.34], // e
    [0.0, -0.19, 1.39], // f
    [0.0, -0.22, 1.27], // g
    [0.0, -0.24, 1.25], // h
    [0.0, -0.22, 0.93], // i
    [0.0, -0.15, 1.20], // j
    [0.0, -0.20, 1.09], // k
    [0.0, -0.20, 1.16], // l
    [0.0, -0.26, 1.18], // m
    [0.0, -0.22, 0.99], // n
    [0.0, -0.19, 1.06], // o
    [0.0, -0.14, 1.21], // p
    [0.0, -0.19, 1.28], // q
    [0.0, -0.10, 1.18], // r
];

/// Muzzle points in **blaster-local** units (under `held_blaster`). See blaster kit README.
pub const BLASTER_MUZZLE_POINTS: &[&[[f32; 3]]] = &[
    &[[0.0, 0.053333, -0.373333]],                                // a
    &[[0.0, 0.013333, -0.26]],                                    // b
    &[[0.0, 0.02, -0.24]],                                        // c
    &[[0.0, 0.056667, -0.456667]],                                // d
    &[[-0.046667, 0.026667, 0.0]],                                // e
    &[[0.0, 0.046667, -0.653333]],                                // f
    &[[0.0, 0.08, -0.353333]],                                    // g
    &[[0.0, 0.026667, -0.32]],                                    // h
    &[[0.0, 0.026667, -0.26], [0.0, -0.046667, -0.26]],           // i
    &[[0.03, 0.093333, -0.303333], [-0.03, 0.093333, -0.303333]], // j
    &[[0.0, -0.013333, -0.233333]],                               // k
    &[[0.066667, 0.04, -0.28], [-0.066667, 0.04, -0.28]],         // l
    &[[0.0, 0.073333, -0.313333]],                                // m
    &[[0.0, 0.066667, -0.32]],                                    // n
    &[
        [0.033333, 0.04, -0.193333],
        [-0.033333, 0.04, -0.193333],
        [0.033333, -0.026667, -0.193333],
        [-0.033333, -0.026667, -0.193333],
    ], // o
    &[[0.0, 0.063333, -0.43], [0.0, 0.0, -0.43]],                 // p
    &[[0.0, 0.06, -0.36], [0.0, -0.086667, -0.36]],               // q
    &[[0.0, 0.086667, -0.42]],                                    // r
];

/// Weapon grip matrix **G** for a blaster letter (feature 037).
#[inline]
pub fn weapon_grip(letter_index: usize) -> Mat4 {
    Mat4::from_translation(Vec3::from_array(BLASTER_GRIP_G[letter_index]))
}

/// Blaster-local muzzle points for a letter (feature 037 / 012).
#[inline]
pub fn muzzle_locals(letter_index: usize) -> &'static [[f32; 3]] {
    BLASTER_MUZZLE_POINTS[letter_index]
}

/// Primary muzzle in blaster-local units (muzzle FX / fire origin; not an aim basis — 015).
#[allow(dead_code)]
pub fn primary_muzzle_offset(letter_index: usize) -> Vec3 {
    Vec3::from_array(muzzle_locals(letter_index)[0])
}

/// World-space image of every blaster-local muzzle under a held root (037).
pub fn muzzle_world_points(held_blaster: Mat4, letter_index: usize) -> impl Iterator<Item = Vec3> {
    muzzle_locals(letter_index)
        .iter()
        .map(move |&p| held_blaster.transform_point3(Vec3::from_array(p)))
}

pub fn letter_index(letter: u8) -> Result<usize, String> {
    if (b'a'..=b'r').contains(&letter) {
        Ok((letter - b'a') as usize)
    } else {
        Err(format!("kit letter out of range: {}", letter as char))
    }
}

pub fn kit_to_world(placement: Mat4, min_y_kit: f32) -> Mat4 {
    placement
        * Mat4::from_scale(Vec3::splat(CHAR_UNITS_TO_M))
        * Mat4::from_translation(Vec3::new(0.0, -min_y_kit, 0.0))
}

/// Held blaster root (feature 037).
///
/// ```text
/// held_blaster = kit_to_world · arm_right_kit · H_hold · inv(G) · S_blaster
/// ```
///
/// `arm_right_kit` is the current pose arm matrix (hold / aim / sprint loco).
/// `H_hold` is the shared hand socket; `G` is the per-letter weapon grip.
/// The product preserves the armed-hold look and follows the arm under loco.
pub fn held_blaster_root(kit_to_world: Mat4, arm_right_kit: Mat4, letter_index: usize) -> Mat4 {
    let h = hand_socket_hold();
    let g = weapon_grip(letter_index);
    let s = Mat4::from_scale(Vec3::splat(BLASTER_RELATIVE_SCALE));
    kit_to_world * arm_right_kit * h * g.inverse() * s
}

pub async fn load_kenney_core() -> Result<Pack, JsValue> {
    pack::load_pack(KENNEY_CORE_PACK).await
}

#[cfg(test)]
mod held_attach_tests {
    use super::*;

    /// Pre-037 arm-attachment grip (arm-local after holding-right). Frozen for migration checks.
    const LEGACY_GRIP_ARM: [[f32; 3]; 18] = [
        [0.0, -1.14, 0.34],
        [0.0, -1.00, 0.30],
        [0.0, -1.11, 0.20],
        [0.0, -1.11, 0.18],
        [0.0, -2.34, 0.22],
        [0.0, -1.39, 0.19],
        [0.0, -1.27, 0.22],
        [0.0, -1.25, 0.24],
        [0.0, -0.93, 0.22],
        [0.0, -1.20, 0.15],
        [0.0, -1.09, 0.20],
        [0.0, -1.16, 0.20],
        [0.0, -1.18, 0.26],
        [0.0, -0.99, 0.22],
        [0.0, -1.06, 0.19],
        [0.0, -1.21, 0.14],
        [0.0, -1.28, 0.19],
        [0.0, -1.18, 0.10],
    ];

    /// Pre-037 muzzle points in the same arm-attachment frame.
    const LEGACY_MUZZLE_ARM: &[&[[f32; 3]]] = &[
        &[[0.0, -1.7, 0.42]],
        &[[0.0, -1.39, 0.32]],
        &[[0.0, -1.47, 0.23]],
        &[[0.0, -1.795, 0.265]],
        &[[0.07, -2.34, 0.26]],
        &[[0.0, -2.37, 0.26]],
        &[[0.0, -1.8, 0.34]],
        &[[0.0, -1.73, 0.28]],
        &[[0.0, -1.32, 0.26], [0.0, -1.32, 0.15]],
        &[[-0.045, -1.655, 0.29], [0.045, -1.655, 0.29]],
        &[[0.0, -1.44, 0.18]],
        &[[-0.1, -1.58, 0.26], [0.1, -1.58, 0.26]],
        &[[0.0, -1.65, 0.37]],
        &[[0.0, -1.47, 0.32]],
        &[
            [-0.05, -1.35, 0.25],
            [0.05, -1.35, 0.25],
            [-0.05, -1.35, 0.15],
            [0.05, -1.35, 0.15],
        ],
        &[[0.0, -1.855, 0.235], [0.0, -1.855, 0.14]],
        &[[0.0, -1.82, 0.28], [0.0, -1.82, 0.06]],
        &[[0.0, -1.81, 0.23]],
    ];

    fn legacy_held(k2w: Mat4, arm: Mat4, letter: usize) -> Mat4 {
        let grip = Vec3::from_array(LEGACY_GRIP_ARM[letter]);
        k2w * arm
            * Mat4::from_translation(grip)
            * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
            * Mat4::from_rotation_y(std::f32::consts::PI)
            * Mat4::from_scale(Vec3::splat(BLASTER_RELATIVE_SCALE))
    }

    fn approx_mat4(a: Mat4, b: Mat4, eps: f32) -> bool {
        a.to_cols_array()
            .iter()
            .zip(b.to_cols_array().iter())
            .all(|(x, y)| (x - y).abs() < eps)
    }

    #[test]
    fn held_root_matches_legacy_hold_product() {
        let k2w = kit_to_world(Mat4::from_translation(Vec3::new(3.0, 0.0, -2.0)), 0.0);
        // Non-trivial arm (hold-like + extra pitch).
        let arm = Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
            * Mat4::from_rotation_x(-0.35)
            * Mat4::from_translation(Vec3::new(-0.4, 1.8, -0.1));
        for i in 0..18 {
            let neu = held_blaster_root(k2w, arm, i);
            let old = legacy_held(k2w, arm, i);
            assert!(
                approx_mat4(neu, old, 1e-5),
                "letter {} held root diverged from legacy hold product",
                (b'a' + i as u8) as char
            );
        }
    }

    #[test]
    fn muzzle_world_matches_legacy_arm_frame() {
        let k2w = kit_to_world(Mat4::IDENTITY, 0.0);
        let arm = Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        for i in 0..18 {
            let held = held_blaster_root(k2w, arm, i);
            let neu: Vec<Vec3> = muzzle_world_points(held, i).collect();
            let legacy: Vec<Vec3> = LEGACY_MUZZLE_ARM[i]
                .iter()
                .map(|&p| k2w.transform_point3(arm.transform_point3(Vec3::from_array(p))))
                .collect();
            assert_eq!(neu.len(), legacy.len());
            for (n, l) in neu.iter().zip(legacy.iter()) {
                assert!(
                    n.distance(*l) < 1e-4,
                    "letter {} muzzle {:?} vs legacy {:?}",
                    (b'a' + i as u8) as char,
                    n,
                    l
                );
            }
        }
    }
}
