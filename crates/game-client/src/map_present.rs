//! Cooked map load and draw (064 shipment container, 066 solids, 070/072/073 foot patches, 082 ground, 083 gravel, 084 cement, 085 grass, 086 container albedo, 087 closed door hardware, 088 lit map solids, 089 morning light, 090 rail corridor, 091 stationed train, 092 train collide, 093 parallel rail, 094 yard tractor, 095 open-door container shell).

#[cfg(target_arch = "wasm32")]
use glam::Mat4;
use glam::Vec3;
use serde::Deserialize;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(target_arch = "wasm32")]
use crate::mesh::{self, LightPlate, SolidUvLayout, UnlitMeshGpu, UnlitMeshLayout};
#[cfg(target_arch = "wasm32")]
use crate::pack;
use game_sim::{MapBox, MapRamp, MapWorld};

pub const MAP_A_PACK: &str = "maps-a";

/// Map **a** morning key + ambient (**089**). Peak ≈ 0.76 — headroom for later locals.
#[cfg(target_arch = "wasm32")]
pub const MAP_A_MORNING_LIGHT: LightPlate = LightPlate {
    light_dir: Vec3::new(0.82, 0.22, 0.38),
    key_color: [0.40, 0.37, 0.32],
    ambient: [0.36, 0.39, 0.45],
};

/// Flat morning sky stand-in while map **a** is drawn (**089**).
#[cfg(target_arch = "wasm32")]
pub const MAP_A_CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.30,
    g: 0.38,
    b: 0.48,
    a: 1.0,
};

/// Flat fallback when container albedos fail (086).
const CONTAINER_FALLBACK_COLOR: [f32; 4] = [0.55, 0.62, 0.72, 1.0];
const BOX_COLOR: [f32; 4] = [0.72, 0.58, 0.42, 1.0];
const RAMP_COLOR: [f32; 4] = [0.48, 0.66, 0.52, 1.0];
/// Visual slab half-height for ground and foot pads (present only — not collide).
const FOOT_PATCH_HALF_Y: f32 = 0.02;
/// World metres per gravel albedo tile (083).
const GRAVEL_METRES_PER_TILE: f32 = 1.5;
/// World metres per cement albedo tile (084).
const CEMENT_METRES_PER_TILE: f32 = 1.5;
/// World metres per grass albedo tile (085).
const GRASS_METRES_PER_TILE: f32 = 1.5;
/// Exactly two side-albedo tiles span the container height (087).
const CONTAINER_SIDE_TILES_HIGH: f32 = 2.0;
/// Shell skin thickness for open-door collide (095).
const CONTAINER_SHELL_T: f32 = 0.08;
/// Front mouth frame centre (same as hardware).
const CONTAINER_FRONT_FRAME_Z: f32 = 0.035;
const CONTAINER_FRONT_FRAME_HALF_Z: f32 = 0.04;
/// Side post centre offset past container half-x, and post half-extent in X.
const CONTAINER_FRONT_POST_X: f32 = 0.035;
const CONTAINER_FRONT_POST_HALF_X: f32 = 0.04;
const CONTAINER_FRONT_PIN_R: f32 = 0.025;
/// Pin on frame inside-edge X (matches closed-end cylinders).
const CONTAINER_FRONT_PIN_X_INSET: f32 = 0.010;
/// Asymmetric open (outward). Left wide, right ajar.
const CONTAINER_LEFT_DOOR_OPEN_RAD: f32 = 0.30; // ~17°
const CONTAINER_RIGHT_DOOR_OPEN_RAD: f32 = 1.85; // ~106°
/// Hinge door-strap half-thickness (095 bar).
const CONTAINER_HINGE_STRAP_HZ: f32 = 0.006;
/// Strap along door face: half-length, half-Y at pin, half-Y at tip (tapers away from pin).
const CONTAINER_HINGE_STRAP_HALF_LEN: f32 = 0.105;
const CONTAINER_HINGE_STRAP_HALF_Y_PIN: f32 = 0.055;
const CONTAINER_HINGE_STRAP_HALF_Y_TIP: f32 = 0.035;
/// Pin barrel half-height — matches strap width at the pin.
const CONTAINER_HINGE_PIN_HALF_Y: f32 = CONTAINER_HINGE_STRAP_HALF_Y_PIN;
const CONTAINER_FRAME_COLOR: [f32; 4] = [0.39, 0.17, 0.14, 1.0];
const CONTAINER_FRAME_COLOR_GREEN: [f32; 4] = [0.24, 0.38, 0.27, 1.0];
const CONTAINER_GASKET_COLOR: [f32; 4] = [0.025, 0.028, 0.026, 1.0];
const CONTAINER_HARDWARE_COLOR: [f32; 4] = [0.42, 0.44, 0.41, 1.0];

fn paint_red() -> String {
    "red".into()
}

/// Side / door pack ids for a container paint (`red` default, `green`).
fn container_albedo_ids(paint: &str) -> (&'static str, &'static str) {
    match paint {
        "green" => ("container-side-green.albedo", "container-door-green.albedo"),
        _ => ("container-side-red.albedo", "container-door-red.albedo"),
    }
}

fn container_frame_paint(paint: &str) -> [f32; 4] {
    match paint {
        "green" => CONTAINER_FRAME_COLOR_GREEN,
        _ => CONTAINER_FRAME_COLOR,
    }
}

/// Swivel pin XZ — rests on the Front frame outer face, on the inside edge.
fn front_hinge_pin_xz(half: Vec3, side: f32) -> (f32, f32) {
    let front_z = -half.z - CONTAINER_FRONT_FRAME_Z;
    let frame_outer_z = front_z - CONTAINER_FRONT_FRAME_HALF_Z;
    (
        side * (half.x - CONTAINER_FRONT_PIN_X_INSET),
        frame_outer_z - CONTAINER_FRONT_PIN_R,
    )
}

#[derive(Clone, Copy)]
struct FrontLeafPose {
    center: Vec3,
    yaw: f32,
    half: Vec3,
}

/// Doors sized to the mouth frame clear; hinged about the pin (095).
fn front_leaf_poses(half: Vec3) -> [FrontLeafPose; 2] {
    let leaf_half = leaf_half_extents(half);
    [
        front_leaf_pose(half, leaf_half, -1.0, CONTAINER_LEFT_DOOR_OPEN_RAD),
        front_leaf_pose(half, leaf_half, 1.0, CONTAINER_RIGHT_DOOR_OPEN_RAD),
    ]
}

fn front_leaf_pose(half: Vec3, leaf_half: Vec3, side: f32, open_rad: f32) -> FrontLeafPose {
    let (pin_x, pin_z) = front_hinge_pin_xz(half, side);
    let pin = Vec3::new(pin_x, 0.0, pin_z);
    let front_z = -half.z - CONTAINER_FRONT_FRAME_Z;
    let frame_outer_z = front_z - CONTAINER_FRONT_FRAME_HALF_Z;
    let door_center_x = side * leaf_half.x;
    let closed_z = frame_outer_z + leaf_half.z;
    let closed_offset = Vec3::new(door_center_x - pin_x, 0.0, closed_z - pin_z);
    let yaw = -side * open_rad;
    let (s, c) = yaw.sin_cos();
    let rotated = Vec3::new(
        c * closed_offset.x + s * closed_offset.z,
        closed_offset.y,
        -s * closed_offset.x + c * closed_offset.z,
    );
    FrontLeafPose {
        center: pin + rotated,
        yaw,
        half: leaf_half,
    }
}

fn frame_inner_half_x(half: Vec3) -> f32 {
    half.x + CONTAINER_FRONT_POST_X - CONTAINER_FRONT_POST_HALF_X
}

fn leaf_half_extents(half: Vec3) -> Vec3 {
    let leaf_hx = frame_inner_half_x(half) * 0.5;
    let leaf_hy = half.y - 0.11;
    Vec3::new(leaf_hx, leaf_hy, 0.02)
}

/// Closed rear leaves — outer face flush with the rear frame (095).
fn rear_leaf_poses(half: Vec3) -> [FrontLeafPose; 2] {
    let leaf_half = leaf_half_extents(half);
    let rear_frame_outer_z = half.z + CONTAINER_FRONT_FRAME_Z + CONTAINER_FRONT_FRAME_HALF_Z;
    let closed_z = rear_frame_outer_z - leaf_half.z;
    [
        FrontLeafPose {
            center: Vec3::new(-leaf_half.x, 0.0, closed_z),
            yaw: 0.0,
            half: leaf_half,
        },
        FrontLeafPose {
            center: Vec3::new(leaf_half.x, 0.0, closed_z),
            yaw: 0.0,
            half: leaf_half,
        },
    ]
}

/// Open-door shell volumes from the map-def outer AABB (095). Floor first.
fn shipment_container_shell(center: Vec3, half: Vec3, yaw: f32) -> Vec<MapBox> {
    let t = CONTAINER_SHELL_T;
    let place = |local: Vec3, box_half: Vec3, local_yaw: f32| {
        let (s, c) = yaw.sin_cos();
        MapBox {
            center: center
                + Vec3::new(
                    c * local.x + s * local.z,
                    local.y,
                    -s * local.x + c * local.z,
                ),
            half: box_half,
            yaw: yaw + local_yaw,
        }
    };
    let mut out = Vec::with_capacity(11);
    out.push(place(
        Vec3::new(0.0, -half.y + t * 0.5, 0.0),
        Vec3::new(half.x, t * 0.5, half.z),
        0.0,
    ));
    out.push(place(
        Vec3::new(0.0, half.y - t * 0.5, 0.0),
        Vec3::new(half.x, t * 0.5, half.z),
        0.0,
    ));
    out.push(place(
        Vec3::new(-half.x + t * 0.5, 0.0, 0.0),
        Vec3::new(t * 0.5, half.y, half.z),
        0.0,
    ));
    out.push(place(
        Vec3::new(half.x - t * 0.5, 0.0, 0.0),
        Vec3::new(t * 0.5, half.y, half.z),
        0.0,
    ));
    // Closed rear (+Z).
    out.push(place(
        Vec3::new(0.0, 0.0, half.z - t * 0.5),
        Vec3::new(half.x, half.y, t * 0.5),
        0.0,
    ));
    let jamb = t * 0.5;
    let mouth_z = -half.z + t * 0.5;
    // Front jambs / header / sill (−Z mouth).
    out.push(place(
        Vec3::new(-half.x + t + jamb, 0.0, mouth_z),
        Vec3::new(jamb, half.y - t, t * 0.5),
        0.0,
    ));
    out.push(place(
        Vec3::new(half.x - t - jamb, 0.0, mouth_z),
        Vec3::new(jamb, half.y - t, t * 0.5),
        0.0,
    ));
    out.push(place(
        Vec3::new(0.0, half.y - t * 0.5, mouth_z),
        Vec3::new(half.x - t, t * 0.5, t * 0.5),
        0.0,
    ));
    out.push(place(
        Vec3::new(0.0, -half.y + t * 0.5, mouth_z),
        Vec3::new(half.x - t, t * 0.5, t * 0.5),
        0.0,
    ));
    for leaf in front_leaf_poses(half) {
        out.push(place(leaf.center, leaf.half, leaf.yaw));
    }
    out
}

#[cfg(target_arch = "wasm32")]
fn container_door_hardware(half: Vec3) -> Vec<(mesh::CpuPrim, [f32; 4], &'static str)> {
    let cuboid = |center: Vec3, half: Vec3, color| {
        (mesh::box_prim(half, color), Mat4::from_translation(center))
    };
    let cylinder = |center: Vec3, radius: f32, half_height: f32, rotation: Mat4, color| {
        (
            mesh::cylinder_y_prim(radius, half_height, 10, color),
            Mat4::from_translation(center) * rotation,
        )
    };

    let mut frame = Vec::new();
    let mut gasket = Vec::new();
    let mut hardware = Vec::new();

    for x in [-half.x - 0.035, half.x + 0.035] {
        for y in [-half.y + 0.055, half.y - 0.055] {
            frame.push(cuboid(
                Vec3::new(x, y, 0.0),
                Vec3::new(0.04, 0.055, half.z + 0.075),
                CONTAINER_FRAME_COLOR,
            ));
        }
        for z in [-half.z - 0.035, half.z + 0.035] {
            frame.push(cuboid(
                Vec3::new(x, 0.0, z),
                Vec3::new(0.04, half.y - 0.11, 0.04),
                CONTAINER_FRAME_COLOR,
            ));
        }
    }

    // Closed rear (+Z) only — Front (−Z) is the open mouth (095).
    let z_sign = 1.0_f32;
    let face_z = z_sign * half.z;
    let out = |d: f32| face_z + z_sign * d;

    let frame_z = out(0.035);
    frame.push(cuboid(
        Vec3::new(0.0, half.y - 0.055, frame_z),
        Vec3::new(half.x + 0.075, 0.055, 0.04),
        CONTAINER_FRAME_COLOR,
    ));
    frame.push(cuboid(
        Vec3::new(0.0, -half.y + 0.055, frame_z),
        Vec3::new(half.x + 0.075, 0.055, 0.04),
        CONTAINER_FRAME_COLOR,
    ));
    frame.push(cuboid(
        Vec3::new(-half.x - 0.035, 0.0, frame_z),
        Vec3::new(0.04, half.y - 0.11, 0.04),
        CONTAINER_FRAME_COLOR,
    ));
    frame.push(cuboid(
        Vec3::new(half.x + 0.035, 0.0, frame_z),
        Vec3::new(0.04, half.y - 0.11, 0.04),
        CONTAINER_FRAME_COLOR,
    ));
    frame.push(cuboid(
        Vec3::new(0.0, 0.0, out(0.155)),
        Vec3::new(0.115, 0.08, 0.03),
        CONTAINER_FRAME_COLOR,
    ));

    let gasket_z = out(0.082);
    let gasket_half_thickness = 0.020;
    let gasket_half_z = 0.0025;
    let gasket_edge_x = half.x - 0.035;
    let gasket_edge_y = half.y - 0.115;
    gasket.push(cuboid(
        Vec3::new(0.0, gasket_edge_y, gasket_z),
        Vec3::new(
            gasket_edge_x + gasket_half_thickness,
            gasket_half_thickness,
            gasket_half_z,
        ),
        CONTAINER_GASKET_COLOR,
    ));
    gasket.push(cuboid(
        Vec3::new(0.0, -gasket_edge_y, gasket_z),
        Vec3::new(
            gasket_edge_x + gasket_half_thickness,
            gasket_half_thickness,
            gasket_half_z,
        ),
        CONTAINER_GASKET_COLOR,
    ));
    gasket.push(cuboid(
        Vec3::new(-gasket_edge_x, 0.0, gasket_z),
        Vec3::new(
            gasket_half_thickness,
            gasket_edge_y + gasket_half_thickness,
            gasket_half_z,
        ),
        CONTAINER_GASKET_COLOR,
    ));
    gasket.push(cuboid(
        Vec3::new(gasket_edge_x, 0.0, gasket_z),
        Vec3::new(
            gasket_half_thickness,
            gasket_edge_y + gasket_half_thickness,
            gasket_half_z,
        ),
        CONTAINER_GASKET_COLOR,
    ));
    gasket.push(cuboid(
        Vec3::new(0.0, 0.0, gasket_z),
        Vec3::new(
            gasket_half_thickness,
            gasket_edge_y + gasket_half_thickness,
            gasket_half_z,
        ),
        CONTAINER_GASKET_COLOR,
    ));

    let hardware_z = out(0.125);
    let inner_rod_x = half.x * 0.23;
    let outer_rod_x = half.x * 0.56;
    let rod_xs = [-outer_rod_x, -inner_rod_x, inner_rod_x, outer_rod_x];
    for x in rod_xs {
        hardware.push(cylinder(
            Vec3::new(x, 0.0, hardware_z),
            0.026,
            half.y - 0.16,
            Mat4::IDENTITY,
            CONTAINER_HARDWARE_COLOR,
        ));
        for y in [-half.y + 0.13, half.y - 0.13] {
            hardware.push(cuboid(
                Vec3::new(x, y, out(0.100)),
                Vec3::new(0.075, 0.06, 0.045),
                CONTAINER_HARDWARE_COLOR,
            ));
            hardware.push(cuboid(
                Vec3::new(x + 0.045, y.signum() * (half.y - 0.19), hardware_z),
                Vec3::new(0.065, 0.025, 0.03),
                CONTAINER_HARDWARE_COLOR,
            ));
        }
    }

    let linkage_half = (outer_rod_x - inner_rod_x) * 0.5;
    let linkage_x = (outer_rod_x + inner_rod_x) * 0.5;
    for sign in [-1.0_f32, 1.0] {
        hardware.push(cylinder(
            Vec3::new(sign * linkage_x, -half.y * 0.36, out(0.140)),
            0.022,
            linkage_half,
            Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2),
            CONTAINER_HARDWARE_COLOR,
        ));
    }

    // Closed rear hinges: pin on frame + door strap (095).
    let rear_frame_z = out(CONTAINER_FRONT_FRAME_Z);
    let rear_frame_outer_z = rear_frame_z + CONTAINER_FRONT_FRAME_HALF_Z;
    let pin_r = CONTAINER_FRONT_PIN_R;
    for sign in [-1.0_f32, 1.0] {
        let pin_x = sign * (half.x - CONTAINER_FRONT_PIN_X_INSET);
        let pin_z = rear_frame_outer_z + pin_r;
        for y in [-half.y * 0.72, -half.y * 0.24, half.y * 0.24, half.y * 0.72] {
            frame.push(cylinder(
                Vec3::new(pin_x, y, pin_z),
                pin_r,
                CONTAINER_HINGE_PIN_HALF_Y,
                Mat4::IDENTITY,
                CONTAINER_FRAME_COLOR,
            ));
        }
    }
    for leaf in rear_leaf_poses(half) {
        let leaf_xf = Mat4::from_translation(leaf.center) * Mat4::from_rotation_y(leaf.yaw);
        let side = if leaf.center.x < 0.0 { -1.0_f32 } else { 1.0 };
        for y in [-half.y * 0.72, -half.y * 0.24, half.y * 0.24, half.y * 0.72] {
            frame.push(door_hinge_strap(leaf_xf, leaf.half, side, y, 1.0));
        }
    }

    // Open Front (−Z): mouth frame + hinge pins (doors are separate batches).
    let front_z = -half.z - CONTAINER_FRONT_FRAME_Z;
    frame.push(cuboid(
        Vec3::new(0.0, half.y - 0.055, front_z),
        Vec3::new(half.x + 0.075, 0.055, CONTAINER_FRONT_FRAME_HALF_Z),
        CONTAINER_FRAME_COLOR,
    ));
    frame.push(cuboid(
        Vec3::new(0.0, -half.y + 0.055, front_z),
        Vec3::new(half.x + 0.075, 0.055, CONTAINER_FRONT_FRAME_HALF_Z),
        CONTAINER_FRAME_COLOR,
    ));
    frame.push(cuboid(
        Vec3::new(-half.x - CONTAINER_FRONT_POST_X, 0.0, front_z),
        Vec3::new(
            CONTAINER_FRONT_POST_HALF_X,
            half.y - 0.11,
            CONTAINER_FRONT_FRAME_HALF_Z,
        ),
        CONTAINER_FRAME_COLOR,
    ));
    frame.push(cuboid(
        Vec3::new(half.x + CONTAINER_FRONT_POST_X, 0.0, front_z),
        Vec3::new(
            CONTAINER_FRONT_POST_HALF_X,
            half.y - 0.11,
            CONTAINER_FRONT_FRAME_HALF_Z,
        ),
        CONTAINER_FRAME_COLOR,
    ));
    for sign in [-1.0_f32, 1.0] {
        let (pin_x, pin_z) = front_hinge_pin_xz(half, sign);
        for y in [-half.y * 0.72, -half.y * 0.24, half.y * 0.24, half.y * 0.72] {
            frame.push(cylinder(
                Vec3::new(pin_x, y, pin_z),
                CONTAINER_FRONT_PIN_R,
                CONTAINER_HINGE_PIN_HALF_Y,
                Mat4::IDENTITY,
                CONTAINER_FRAME_COLOR,
            ));
        }
    }
    for side in [-1.0_f32, 1.0] {
        let open = if side < 0.0 {
            CONTAINER_LEFT_DOOR_OPEN_RAD
        } else {
            CONTAINER_RIGHT_DOOR_OPEN_RAD
        };
        let leaf = front_leaf_pose(half, leaf_half_extents(half), side, open);
        let leaf_xf = Mat4::from_translation(leaf.center) * Mat4::from_rotation_y(leaf.yaw);
        for y in [-half.y * 0.72, -half.y * 0.24, half.y * 0.24, half.y * 0.72] {
            frame.push(door_hinge_strap(leaf_xf, leaf.half, side, y, -1.0));
        }
    }

    vec![
        (
            mesh::merge_transformed_prims(frame, CONTAINER_FRAME_COLOR),
            CONTAINER_FRAME_COLOR,
            "map-a-container-door-frame",
        ),
        (
            mesh::merge_transformed_prims(gasket, CONTAINER_GASKET_COLOR),
            CONTAINER_GASKET_COLOR,
            "map-a-container-door-gasket",
        ),
        (
            mesh::merge_transformed_prims(hardware, CONTAINER_HARDWARE_COLOR),
            CONTAINER_HARDWARE_COLOR,
            "map-a-container-door-hardware",
        ),
    ]
}

/// Door-face hinge strap. `z_out` +1 rear / −1 front.
#[cfg(target_arch = "wasm32")]
fn door_hinge_strap(
    leaf_xf: Mat4,
    leaf_half: Vec3,
    side: f32,
    y: f32,
    z_out: f32,
) -> (mesh::CpuPrim, Mat4) {
    let strap_hz = CONTAINER_HINGE_STRAP_HZ;
    let strap_local = Vec3::new(
        side * (leaf_half.x - CONTAINER_HINGE_STRAP_HALF_LEN),
        y,
        z_out * (leaf_half.z + strap_hz),
    );
    // Prim pin is at local −X; yaw 180° on the +X leaf so pin points toward the hinge.
    let pin_yaw = if side > 0.0 {
        std::f32::consts::PI
    } else {
        0.0
    };
    let local = leaf_xf * Mat4::from_translation(strap_local) * Mat4::from_rotation_y(pin_yaw);
    (
        mesh::hinge_strap_prim(
            CONTAINER_HINGE_STRAP_HALF_LEN,
            CONTAINER_HINGE_STRAP_HALF_Y_PIN,
            CONTAINER_HINGE_STRAP_HALF_Y_TIP,
            strap_hz,
            CONTAINER_FRAME_COLOR,
        ),
        local,
    )
}

/// Front door leaves: hinged about the pin, sized to the mouth frame (095).
#[cfg(target_arch = "wasm32")]
fn container_open_front_leaves(half: Vec3) -> Vec<(mesh::CpuPrim, Mat4)> {
    front_leaf_poses(half)
        .into_iter()
        .map(|leaf| {
            let prim = mesh::box_prim(leaf.half, [1.0, 1.0, 1.0, 1.0]);
            let local = Mat4::from_translation(leaf.center) * Mat4::from_rotation_y(leaf.yaw);
            (prim, local)
        })
        .collect()
}

/// Closed rear leaves flush with the rear frame outer face (095).
#[cfg(target_arch = "wasm32")]
fn container_closed_rear_leaves(half: Vec3) -> Vec<(mesh::CpuPrim, Mat4)> {
    rear_leaf_poses(half)
        .into_iter()
        .map(|leaf| {
            let prim = mesh::box_prim(leaf.half, [1.0, 1.0, 1.0, 1.0]);
            let local = Mat4::from_translation(leaf.center) * Mat4::from_rotation_y(leaf.yaw);
            (prim, local)
        })
        .collect()
}

/// Interior floor deck — same skin as the roof, on the shell floor (095).
#[cfg(target_arch = "wasm32")]
fn container_interior_floor(half: Vec3) -> (mesh::CpuPrim, Mat4) {
    let t = CONTAINER_SHELL_T;
    let floor_half = Vec3::new(half.x, t * 0.5, half.z);
    let prim = mesh::box_prim(floor_half, [1.0, 1.0, 1.0, 1.0]);
    let local = Mat4::from_translation(Vec3::new(0.0, -half.y + t * 0.5, 0.0));
    (prim, local)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootKind {
    Gravel,
    Cement,
    WetCement,
    Grass,
    Steel,
}

impl FootKind {
    /// Shared kind → albedo for ground and foot pads (082).
    pub fn albedo(self) -> [f32; 4] {
        match self {
            Self::Gravel => [1.0, 1.0, 1.0, 1.0],
            Self::Cement => [1.0, 1.0, 1.0, 1.0],
            Self::WetCement => [0.42, 0.48, 0.52, 1.0],
            Self::Grass => [1.0, 1.0, 1.0, 1.0],
            Self::Steel => [1.0, 1.0, 1.0, 1.0],
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FootPatch {
    kind: FootKind,
    center_x: f32,
    center_z: f32,
    half_x: f32,
    half_z: f32,
    /// XZ yaw; `0` = axis-aligned (map-def pads / rail strips).
    yaw: f32,
    draw: bool,
}

impl FootPatch {
    fn contains(self, x: f32, z: f32) -> bool {
        let dx = x - self.center_x;
        let dz = z - self.center_z;
        let (s, c) = self.yaw.sin_cos();
        // Same local frame as `MapBox` (inverse of kit world_from_local).
        let lx = c * dx - s * dz;
        let lz = s * dx + c * dz;
        lx.abs() <= self.half_x && lz.abs() <= self.half_z
    }
}

/// Present-only foot surface patches (`gravel` / `cement` / `wet_cement` / `grass` / `steel`). Outside → gravel.
#[derive(Clone, Debug, Default)]
pub struct FootSurfaces {
    patches: Vec<FootPatch>,
}

impl FootSurfaces {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn at(&self, x: f32, z: f32) -> FootKind {
        let mut kind = FootKind::Gravel;
        for p in &self.patches {
            if p.contains(x, z) {
                kind = p.kind;
            }
        }
        kind
    }
}

#[derive(Debug, Deserialize)]
struct MapDef {
    ground: GroundDef,
    #[serde(default)]
    rail: Option<RailDef>,
    #[serde(default)]
    train: Option<TrainDef>,
    #[serde(default)]
    shipment_containers: Vec<SolidBoxDef>,
    #[serde(default)]
    boxes: Vec<SolidBoxDef>,
    #[serde(default)]
    ramp: Option<RampDef>,
    #[serde(default)]
    foot_patches: Vec<FootPatchDef>,
}

/// Straight east–west rail corridor (090; parallel tracks 093). Present only — no collide.
#[derive(Debug, Deserialize)]
struct RailDef {
    /// World-Z centerlines for each parallel track (map **a**: home −8, north twin −4.8).
    centerlines_z: Vec<f32>,
    x_min: f32,
    x_max: f32,
    /// Along-track spacing of wooden tracks in world metres; metal segments use half this.
    /// Keep in lockstep with `scale` (kit sleeper length × scale).
    stride: f32,
    /// Yaw so kit track-forward (+Z) follows the corridor (map **a**: +π/2 → world +X).
    yaw: f32,
    /// Uniform kit → world scale for track atoms.
    #[serde(default = "kit_scale_one")]
    scale: f32,
}

/// Parked freight consist on the rail corridor (091 draw; 092 collide/support).
#[derive(Debug, Deserialize)]
struct TrainDef {
    centerline_z: f32,
    mid_x: f32,
    yaw: f32,
    #[serde(default = "kit_scale_one")]
    scale: f32,
    seat_y: f32,
    #[serde(default)]
    loco_z_nudge: f32,
    #[serde(default)]
    unit_gap: f32,
    units: Vec<String>,
    #[serde(default)]
    ground_cargo: Option<GroundCargoDef>,
    #[serde(default)]
    tractor: Option<TractorDef>,
}

/// Ground lumber pile beside the consist (091 pose; 092 half-buried jump pad).
#[derive(Debug, Deserialize)]
struct GroundCargoDef {
    mesh: String,
    beside_unit: usize,
    side_z: f32,
    #[serde(default)]
    along_nudge: f32,
    #[serde(default)]
    yaw_nudge: f32,
    seat_y: f32,
    #[serde(default = "kit_scale_one")]
    scale: f32,
}

/// Yard tractor beside the unload pile (094 parkour mid-step).
#[derive(Debug, Deserialize)]
struct TractorDef {
    mesh: String,
    beside_unit: usize,
    side_z: f32,
    #[serde(default)]
    along_nudge: f32,
    #[serde(default)]
    yaw_nudge: f32,
    seat_y: f32,
    #[serde(default = "kit_scale_one")]
    scale: f32,
}

/// Kit-space collide AABB for a rolling-stock stem (092; flatbed top = deck, not stakes).
/// Tank mid seal band only — east/west domes are nose + rear ramps (**094**). Do **not** use Z
/// here for consist packing; see [`train_unit_along_extent`].
fn train_unit_collide_kit(stem: &str) -> ([f32; 3], [f32; 3]) {
    match stem {
        "train-locomotive-c" => ([-0.75, -0.36, -1.4], [0.68, 1.66, 1.4]),
        "train-carriage-flatbed" => ([-0.55, -0.36, -1.35], [0.55, 0.36, 1.35]),
        "train-carriage-lumber" => ([-0.55, -0.36, -1.35], [0.55, 0.92, 1.35]),
        "train-carriage-tank" => ([-0.69, -0.36, -0.08], [0.69, 1.55, 0.08]),
        _ => ([-0.55, -0.36, -1.35], [0.55, 0.36, 1.35]),
    }
}

/// Along-track kit Z span for consist packing / draw roots (full car length).
fn train_unit_along_extent(stem: &str) -> (f32, f32) {
    match stem {
        "train-locomotive-c" => (-1.4, 1.4),
        _ => (-1.35, 1.35),
    }
}

/// Elevated tank dome half: low at the tip, rises to mid-barrel (**094**).
/// `east_half` — toward lumber (+kit Z); otherwise rear (−kit Z).
fn tank_dome_ramp(
    root: Vec3,
    yaw: f32,
    scale: f32,
    seat_y: f32,
    pad_z: f32,
    east_half: bool,
) -> MapRamp {
    let scale = scale.max(1e-3);
    let half = 0.675_f32;
    let kit_z = if east_half {
        half + pad_z * 0.5
    } else {
        -(half + pad_z * 0.5)
    };
    let (s, c) = yaw.sin_cos();
    let lz = kit_z * scale;
    let center_x = root.x + s * lz;
    let center_z = root.z + c * lz;
    let tip_y = 2.48;
    let mid_y = seat_y + 1.55 * scale;
    MapRamp {
        center_x,
        center_z,
        half_x: 0.69 * scale,
        half_z: (half + pad_z * 0.5) * scale,
        height: (mid_y - tip_y).max(0.1),
        base_y: tip_y,
        // East: rise west (yaw+π). Rear: rise east (consist yaw).
        yaw: if east_half {
            yaw + std::f32::consts::PI
        } else {
            yaw
        },
    }
}

/// Kit-space bounds for the stripped lumber pile (`lumber-cargo`, 092).
fn lumber_cargo_collide_kit() -> ([f32; 3], [f32; 3]) {
    ([-0.48, 0.0, -1.25], [0.48, 0.554, 1.25])
}

/// Kit-space bounds for the yard tractor body / hood band (`tractor`, 094).
fn tractor_collide_kit() -> ([f32; 3], [f32; 3]) {
    ([-0.55, 0.0, -0.95], [0.55, 1.50, 0.95])
}

/// World root for a beside-unit yard prop (cargo / tractor).
fn beside_unit_root(
    train: &TrainDef,
    roots: &[(f32, f32)],
    beside_unit: usize,
    along_nudge: f32,
    side_z: f32,
    seat_y: f32,
) -> Option<Vec3> {
    roots
        .get(beside_unit)
        .map(|&(unit_x, _)| Vec3::new(unit_x + along_nudge, seat_y, train.centerline_z + side_z))
}

/// Along-track kit origins (`x`) and lateral (`z`) for each unit — shared by draw and collide.
fn train_unit_roots(train: &TrainDef) -> Vec<(f32, f32)> {
    let scale = train.scale.max(1e-3);
    let gap = train.unit_gap.max(0.0);
    let extents: Vec<(f32, f32)> = train
        .units
        .iter()
        .map(|stem| train_unit_along_extent(stem))
        .collect();
    let mut total = gap * train.units.len().saturating_sub(1) as f32;
    for &(min_z, max_z) in &extents {
        total += (max_z - min_z) * scale;
    }
    let mut front_tip = train.mid_x + total * 0.5;
    let mut roots = Vec::with_capacity(train.units.len());
    for (i, (stem, &(min_z, max_z))) in train.units.iter().zip(extents.iter()).enumerate() {
        let x = front_tip - max_z * scale;
        let z = if stem == "train-locomotive-c" {
            train.centerline_z + train.loco_z_nudge
        } else {
            train.centerline_z
        };
        roots.push((x, z));
        front_tip = x + min_z * scale;
        if i + 1 < train.units.len() {
            front_tip -= gap;
        }
    }
    roots
}

/// Oriented `MapBox` from a kit AABB (half extents stay local; yaw is preserved).
fn kit_map_box(root: Vec3, yaw: f32, scale: f32, min: [f32; 3], max: [f32; 3]) -> MapBox {
    let scale = scale.max(1e-3);
    let local_center = Vec3::new(
        (min[0] + max[0]) * 0.5 * scale,
        (min[1] + max[1]) * 0.5 * scale,
        (min[2] + max[2]) * 0.5 * scale,
    );
    let half = Vec3::new(
        (max[0] - min[0]) * 0.5 * scale,
        (max[1] - min[1]) * 0.5 * scale,
        (max[2] - min[2]) * 0.5 * scale,
    );
    let (s, c) = yaw.sin_cos();
    let center = root
        + Vec3::new(
            c * local_center.x + s * local_center.z,
            local_center.y,
            -s * local_center.x + c * local_center.z,
        );
    MapBox { center, half, yaw }
}

fn train_collide_boxes(train: &TrainDef) -> Vec<MapBox> {
    train_collide_solids(train).0
}

fn train_collide_solids(train: &TrainDef) -> (Vec<MapBox>, Vec<MapRamp>) {
    let scale = train.scale.max(1e-3);
    let roots = train_unit_roots(train);
    // Seal `unit_gap` — ground support is point-sampled.
    let pad_z = (train.unit_gap * 0.5 + 0.05) / scale;
    let mut boxes = Vec::with_capacity(
        roots.len()
            + usize::from(train.ground_cargo.is_some())
            + usize::from(train.tractor.is_some()),
    );
    let mut ramps = Vec::new();
    for (stem, &(x, z)) in train.units.iter().zip(roots.iter()) {
        let root = Vec3::new(x, train.seat_y, z);
        if stem == "train-carriage-tank" {
            let (min, max) = train_unit_collide_kit(stem);
            boxes.push(kit_map_box(root, train.yaw, scale, min, max));
            ramps.push(tank_dome_ramp(
                root,
                train.yaw,
                scale,
                train.seat_y,
                pad_z,
                true,
            ));
            ramps.push(tank_dome_ramp(
                root,
                train.yaw,
                scale,
                train.seat_y,
                pad_z,
                false,
            ));
            continue;
        }
        let (min, max) = train_unit_collide_kit(stem);
        let min = [min[0], min[1], min[2] - pad_z];
        let max = [max[0], max[1], max[2] + pad_z];
        boxes.push(kit_map_box(root, train.yaw, scale, min, max));
    }
    if let Some(cargo) = &train.ground_cargo {
        if let Some(root) = beside_unit_root(
            train,
            &roots,
            cargo.beside_unit,
            cargo.along_nudge,
            cargo.side_z,
            cargo.seat_y,
        ) {
            let (min, max) = lumber_cargo_collide_kit();
            boxes.push(kit_map_box(
                root,
                train.yaw + cargo.yaw_nudge,
                cargo.scale.max(1e-3),
                min,
                max,
            ));
        }
    }
    if let Some(tractor) = &train.tractor {
        if let Some(root) = beside_unit_root(
            train,
            &roots,
            tractor.beside_unit,
            tractor.along_nudge,
            tractor.side_z,
            tractor.seat_y,
        ) {
            let (min, max) = tractor_collide_kit();
            boxes.push(kit_map_box(
                root,
                train.yaw + tractor.yaw_nudge,
                tractor.scale.max(1e-3),
                min,
                max,
            ));
        }
    }
    (boxes, ramps)
}

fn kit_scale_one() -> f32 {
    1.0
}

#[derive(Debug, Deserialize)]
struct GroundDef {
    position: [f32; 3],
    /// `[half_x, half_z]` footprint on the ground plane.
    half_extents: [f32; 2],
}

#[derive(Debug, Deserialize)]
struct SolidBoxDef {
    position: [f32; 3],
    half_extents: [f32; 3],
    /// Container paint grade: `red` (default) or `green` — selects side/door albedos.
    #[serde(default = "paint_red")]
    paint: String,
    /// Yaw about +Y (radians). Positive = CCW from above.
    #[serde(default)]
    yaw: f32,
}

#[derive(Debug, Deserialize)]
struct RampDef {
    position: [f32; 3],
    /// `[half_x, half_z]` footprint.
    half_extents: [f32; 2],
    height: f32,
    #[serde(default)]
    yaw: f32,
}

#[derive(Debug, Deserialize)]
struct FootPatchDef {
    kind: String,
    position: [f32; 3],
    /// `[half_x, half_z]` footprint on the ground plane.
    half_extents: [f32; 2],
}

#[cfg(target_arch = "wasm32")]
pub struct MapGpu {
    mesh: UnlitMeshGpu,
}

#[cfg(target_arch = "wasm32")]
pub enum MapPresentState {
    Idle,
    Loading,
    Ready(MapGpu),
    Failed,
}

#[cfg(target_arch = "wasm32")]
impl MapGpu {
    pub async fn load_map_a(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Result<(Self, MapWorld, FootSurfaces), JsValue> {
        let pack = pack::load_pack(MAP_A_PACK).await?;
        let def_bytes = pack.get("map-a.def").map_err(|e| JsValue::from_str(&e))?;
        let def: MapDef = serde_json::from_slice(def_bytes)
            .map_err(|e| JsValue::from_str(&format!("map-a.def: {e}")))?;

        let world = map_world_from_def(&def);
        let feet = foot_surfaces_from_def(&def).map_err(|e| JsValue::from_str(&e))?;

        let layout = UnlitMeshLayout::create(device, surface_format, sample_count);
        let gpu = layout.upload_ctx(device, queue);
        let mut batches = Vec::new();

        // Ground under override pads (082); textured gravel when albedo loads (083).
        let gravel = FootKind::Gravel.albedo();
        let ground_half = Vec3::new(
            def.ground.half_extents[0],
            FOOT_PATCH_HALF_Y,
            def.ground.half_extents[1],
        );
        let ground_root = Mat4::from_translation(Vec3::new(
            def.ground.position[0],
            FOOT_PATCH_HALF_Y,
            def.ground.position[2],
        ));
        let textured_ground = pack.get("gravel.albedo").and_then(|png| {
            mesh::upload_textured_solid_batch(
                &gpu,
                png,
                mesh::box_prim(ground_half, gravel),
                ground_root,
                gravel,
                GRAVEL_METRES_PER_TILE,
                SolidUvLayout::WorldXz,
                mesh::SolidShading::Lit,
                "map-a-ground",
            )
        });
        let ground_batch = match textured_ground {
            Ok(batch) => batch,
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("map: gravel albedo unusable ({e}); flat ground").into(),
                );
                mesh::upload_solid_batch(
                    &gpu,
                    mesh::box_prim(ground_half, gravel),
                    ground_root,
                    gravel,
                    mesh::SolidShading::Lit,
                    "map-a-ground",
                )
                .map_err(|e| JsValue::from_str(&e))?
            }
        };
        batches.push(ground_batch);

        // Pads sit on the ground top to avoid z-fight with the gravel slab.
        let pad_center_y = FOOT_PATCH_HALF_Y * 3.0;
        for (i, p) in feet.patches.iter().enumerate() {
            if !p.draw {
                continue;
            }
            let color = p.kind.albedo();
            let half = Vec3::new(p.half_x, FOOT_PATCH_HALF_Y, p.half_z);
            let root = Mat4::from_translation(Vec3::new(p.center_x, pad_center_y, p.center_z));
            let label = format!("map-a-foot-{i}");
            let textured = match p.kind {
                FootKind::Cement => Some(("cement.albedo", CEMENT_METRES_PER_TILE)),
                FootKind::Grass => Some(("grass.albedo", GRASS_METRES_PER_TILE)),
                _ => None,
            };
            let batch = if let Some((asset, metres_per_tile)) = textured {
                match pack.get(asset).and_then(|png| {
                    mesh::upload_textured_solid_batch(
                        &gpu,
                        png,
                        mesh::box_prim(half, color),
                        root,
                        color,
                        metres_per_tile,
                        SolidUvLayout::WorldXz,
                        mesh::SolidShading::Lit,
                        &label,
                    )
                }) {
                    Ok(batch) => batch,
                    Err(e) => {
                        web_sys::console::warn_1(
                            &format!("map: {asset} unusable ({e}); flat pad").into(),
                        );
                        mesh::upload_solid_batch(
                            &gpu,
                            mesh::box_prim(half, color),
                            root,
                            color,
                            mesh::SolidShading::Lit,
                            &label,
                        )
                        .map_err(|e| JsValue::from_str(&e))?
                    }
                }
            } else {
                mesh::upload_solid_batch(
                    &gpu,
                    mesh::box_prim(half, color),
                    root,
                    color,
                    mesh::SolidShading::Lit,
                    &label,
                )
                .map_err(|e| JsValue::from_str(&e))?
            };
            batches.push(batch);
        }

        for (ci, container) in def.shipment_containers.iter().enumerate() {
            let half = Vec3::from_array(container.half_extents);
            let root = Mat4::from_translation(Vec3::from_array(container.position))
                * Mat4::from_rotation_y(container.yaw);
            let container_tint = [1.0, 1.0, 1.0, 1.0];
            let (side_albedo, door_albedo) = container_albedo_ids(&container.paint);
            let frame_paint = container_frame_paint(&container.paint);
            let side_metres_per_tile = half.y * 2.0 / CONTAINER_SIDE_TILES_HIGH;
            let upload_container_part = |albedo_id: &str,
                                         group: mesh::BoxFaceGroup,
                                         metres_per_tile: f32,
                                         uv_layout: SolidUvLayout,
                                         label: &str|
             -> Result<_, String> {
                let png = pack.get(albedo_id)?;
                mesh::upload_textured_solid_batch(
                    &gpu,
                    png,
                    mesh::box_face_group_prim(half, container_tint, Some(group)),
                    root,
                    container_tint,
                    metres_per_tile,
                    uv_layout,
                    mesh::SolidShading::Lit,
                    label,
                )
            };
            // Open Front mouth (095); rear is flush door leaves.
            let sides = upload_container_part(
                side_albedo,
                mesh::BoxFaceGroup::Sides,
                side_metres_per_tile,
                SolidUvLayout::BoxFace,
                &format!("map-a-shipment-container-{ci}-sides"),
            );
            let lids = upload_container_part(
                side_albedo,
                mesh::BoxFaceGroup::Lids,
                side_metres_per_tile,
                SolidUvLayout::BoxFace,
                &format!("map-a-shipment-container-{ci}-lids"),
            );
            match (sides, lids) {
                (Ok(sides), Ok(lids)) => {
                    batches.push(sides);
                    batches.push(lids);
                    if let Ok(side_png) = pack.get(side_albedo) {
                        let (floor_prim, floor_local) = container_interior_floor(half);
                        match mesh::upload_textured_solid_batch(
                            &gpu,
                            side_png,
                            floor_prim,
                            root * floor_local,
                            container_tint,
                            side_metres_per_tile,
                            SolidUvLayout::BoxFace,
                            mesh::SolidShading::Lit,
                            &format!("map-a-shipment-container-{ci}-floor"),
                        ) {
                            Ok(batch) => batches.push(batch),
                            Err(e) => web_sys::console::warn_1(
                                &format!("map: container {ci} floor unusable ({e})").into(),
                            ),
                        }
                    }
                    if let Ok(door_png) = pack.get(door_albedo) {
                        for (i, (prim, local)) in
                            container_closed_rear_leaves(half).into_iter().enumerate()
                        {
                            match mesh::upload_textured_solid_batch(
                                &gpu,
                                door_png,
                                prim,
                                root * local,
                                container_tint,
                                1.0,
                                SolidUvLayout::RearDoors,
                                mesh::SolidShading::Lit,
                                &format!("map-a-shipment-container-{ci}-rear-leaf-{i}"),
                            ) {
                                Ok(batch) => batches.push(batch),
                                Err(e) => web_sys::console::warn_1(
                                    &format!("map: container {ci} rear door leaf unusable ({e})")
                                        .into(),
                                ),
                            }
                        }
                        for (i, (prim, local)) in
                            container_open_front_leaves(half).into_iter().enumerate()
                        {
                            match mesh::upload_textured_solid_batch(
                                &gpu,
                                door_png,
                                prim,
                                root * local,
                                container_tint,
                                1.0,
                                SolidUvLayout::RearDoors,
                                mesh::SolidShading::Lit,
                                &format!("map-a-shipment-container-{ci}-open-leaf-{i}"),
                            ) {
                                Ok(batch) => batches.push(batch),
                                Err(e) => web_sys::console::warn_1(
                                    &format!("map: container {ci} open door leaf unusable ({e})")
                                        .into(),
                                ),
                            }
                        }
                    }
                }
                (s, l) => {
                    let err = s
                        .err()
                        .or(l.err())
                        .unwrap_or_else(|| "container albedo".into());
                    web_sys::console::warn_1(
                        &format!("map: container {ci} albedo unusable ({err}); flat container")
                            .into(),
                    );
                    for group in [mesh::BoxFaceGroup::Sides, mesh::BoxFaceGroup::Lids] {
                        batches.push(
                            mesh::upload_solid_batch(
                                &gpu,
                                mesh::box_face_group_prim(
                                    half,
                                    CONTAINER_FALLBACK_COLOR,
                                    Some(group),
                                ),
                                root,
                                CONTAINER_FALLBACK_COLOR,
                                mesh::SolidShading::Lit,
                                &format!("map-a-shipment-container-{ci}"),
                            )
                            .map_err(|e| JsValue::from_str(&e))?,
                        );
                    }
                    let (floor_prim, floor_local) = container_interior_floor(half);
                    batches.push(
                        mesh::upload_solid_batch(
                            &gpu,
                            floor_prim,
                            root * floor_local,
                            CONTAINER_FALLBACK_COLOR,
                            mesh::SolidShading::Lit,
                            &format!("map-a-shipment-container-{ci}-floor"),
                        )
                        .map_err(|e| JsValue::from_str(&e))?,
                    );
                }
            }
            for (prim, color, label) in container_door_hardware(half) {
                // Frame / hinge straps are painted; gasket + latch hardware stay bare.
                let upload_color = if label.contains("frame") {
                    frame_paint
                } else {
                    color
                };
                batches.push(
                    mesh::upload_solid_batch(
                        &gpu,
                        prim,
                        root,
                        upload_color,
                        mesh::SolidShading::Lit,
                        &format!("{label}-{ci}"),
                    )
                    .map_err(|e| JsValue::from_str(&e))?,
                );
            }
        }

        for (i, b) in def.boxes.iter().enumerate() {
            let half = Vec3::from_array(b.half_extents);
            let root = Mat4::from_translation(Vec3::from_array(b.position));
            batches.push(
                mesh::upload_solid_batch(
                    &gpu,
                    mesh::box_prim(half, BOX_COLOR),
                    root,
                    BOX_COLOR,
                    mesh::SolidShading::Lit,
                    &format!("map-a-box-{i}"),
                )
                .map_err(|e| JsValue::from_str(&e))?,
            );
        }

        if let Some(ramp) = &def.ramp {
            let root = Mat4::from_translation(Vec3::new(ramp.position[0], 0.0, ramp.position[2]))
                * Mat4::from_rotation_y(ramp.yaw);
            batches.push(
                mesh::upload_solid_batch(
                    &gpu,
                    mesh::ramp_prim(
                        ramp.half_extents[0],
                        ramp.half_extents[1],
                        ramp.height,
                        RAMP_COLOR,
                    ),
                    root,
                    RAMP_COLOR,
                    mesh::SolidShading::Lit,
                    "map-a-ramp",
                )
                .map_err(|e| JsValue::from_str(&e))?,
            );
        }

        if let Some(rail) = &def.rail {
            match upload_rail_corridor(&gpu, &pack, rail) {
                Ok(rail_batches) => batches.extend(rail_batches),
                Err(e) => {
                    web_sys::console::warn_1(
                        &format!("map: rail corridor unusable ({e}); skipping rail").into(),
                    );
                }
            }
        }

        if let Some(train) = &def.train {
            match upload_stationed_train(&gpu, &pack, train) {
                Ok(train_batch) => batches.push(train_batch),
                Err(e) => {
                    web_sys::console::warn_1(
                        &format!("map: stationed train unusable ({e}); skipping train").into(),
                    );
                }
            }
            match upload_yard_tractor(&gpu, &pack, train) {
                Ok(Some(tractor_batch)) => batches.push(tractor_batch),
                Ok(None) => {}
                Err(e) => {
                    web_sys::console::warn_1(
                        &format!("map: yard tractor unusable ({e}); skipping tractor").into(),
                    );
                }
            }
        }

        Ok((
            Self {
                mesh: layout.finish(batches),
            },
            world,
            feet,
        ))
    }

    pub fn write_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4, light: LightPlate) {
        self.mesh.write_view_proj(queue, view_proj, light);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.mesh.draw(pass);
    }
}

/// Tile Kenney spline atoms along each rail centerline and upload two lit batches (090 / 093).
#[cfg(target_arch = "wasm32")]
fn upload_rail_corridor(
    gpu: &mesh::UploadCtx<'_>,
    pack: &pack::Pack,
    rail: &RailDef,
) -> Result<Vec<mesh::MeshBatch>, String> {
    if rail.centerlines_z.is_empty() {
        return Err("rail.centerlines_z is empty".into());
    }

    let colormap = pack.get("train.colormap")?;
    let segment_glb = pack.get("spline-segment.mesh")?;
    let sleeper_glb = pack.get("spline-track.mesh")?;

    let segment_local = merge_kit_prims(mesh::extract_primitives(segment_glb)?);
    let sleeper_local = merge_kit_prims(mesh::extract_primitives(sleeper_glb)?);

    // Ground present top is `2 * FOOT_PATCH_HALF_Y`. Metal segments sit on it;
    // wooden tracks sit on the segment deck. Segments at 2× sleeper frequency.
    // Instance transform is T·R·S — seating offsets use scaled local Y bands.
    let scale = rail.scale.max(1e-3);
    let ground_top = FOOT_PATCH_HALF_Y * 2.0;
    let sleeper_xs = rail_centers(rail.x_min, rail.x_max, rail.stride);
    let segment_xs = rail_centers(rail.x_min, rail.x_max, rail.stride * 0.5);

    let segment_min_y = prim_min_y(&segment_local);
    let segment_deck_y = prim_deck_y(&segment_local);
    let sleeper_min_y = prim_min_y(&sleeper_local);
    let segment_y = sole_on_plane(ground_top, segment_min_y * scale);
    // Seat wood on the segment deck (below rail-head detail), not mesh max_y.
    let segment_top = if segment_min_y.is_finite() && segment_deck_y.is_finite() {
        segment_y + (segment_deck_y - segment_min_y) * scale
    } else {
        segment_y
    };
    let sleeper_y = sole_on_plane(segment_top, sleeper_min_y * scale);

    let segments = instance_rail_atom(&segment_local, rail, &segment_xs, segment_y);
    let sleepers = instance_rail_atom(&sleeper_local, rail, &sleeper_xs, sleeper_y);

    Ok(vec![
        mesh::upload_batch(
            gpu,
            colormap,
            vec![sleepers],
            Mat4::IDENTITY,
            "map-a-rail-sleepers",
        )?,
        mesh::upload_batch(
            gpu,
            colormap,
            vec![segments],
            Mat4::IDENTITY,
            "map-a-rail-segments",
        )?,
    ])
}

/// Pack the parked freight consist on the corridor and upload one lit batch (091).
#[cfg(target_arch = "wasm32")]
fn upload_stationed_train(
    gpu: &mesh::UploadCtx<'_>,
    pack: &pack::Pack,
    train: &TrainDef,
) -> Result<mesh::MeshBatch, String> {
    if train.units.is_empty() {
        return Err("train.units is empty".into());
    }

    let scale = train.scale.max(1e-3);
    let colormap = pack.get("train.colormap")?;

    let mut piece_locals = Vec::with_capacity(train.units.len());
    for stem in &train.units {
        let id = format!("{stem}.mesh");
        piece_locals.push(merge_kit_prims(mesh::extract_primitives(pack.get(&id)?)?));
    }

    let roots = train_unit_roots(train);
    let mut parts = Vec::with_capacity(piece_locals.len() + 1);
    for (local, &(x, z)) in piece_locals.iter().zip(roots.iter()) {
        let root = kit_tr_s(Vec3::new(x, train.seat_y, z), train.yaw, scale);
        parts.push((local.clone(), root));
    }

    if let Some(cargo) = &train.ground_cargo {
        let i = cargo.beside_unit;
        if i >= roots.len() {
            return Err(format!(
                "ground_cargo.beside_unit {i} out of range ({} units)",
                roots.len()
            ));
        }
        let id = format!("{}.mesh", cargo.mesh);
        let local = merge_kit_prims(mesh::extract_primitives(pack.get(&id)?)?);
        let cargo_scale = cargo.scale.max(1e-3);
        let root = kit_tr_s(
            Vec3::new(
                roots[i].0 + cargo.along_nudge,
                cargo.seat_y,
                train.centerline_z + cargo.side_z,
            ),
            train.yaw + cargo.yaw_nudge,
            cargo_scale,
        );
        parts.push((local, root));
    }

    let color = piece_locals
        .first()
        .map(|p| p.2)
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let merged = mesh::merge_transformed_prims(parts, color);
    mesh::upload_batch(
        gpu,
        colormap,
        vec![merged],
        Mat4::IDENTITY,
        "map-a-stationed-train",
    )
}

/// Park the yard tractor beside the unload pile (094); own car-kit colormap batch.
#[cfg(target_arch = "wasm32")]
fn upload_yard_tractor(
    gpu: &mesh::UploadCtx<'_>,
    pack: &pack::Pack,
    train: &TrainDef,
) -> Result<Option<mesh::MeshBatch>, String> {
    let Some(tractor) = &train.tractor else {
        return Ok(None);
    };
    let roots = train_unit_roots(train);
    let Some(root_pos) = beside_unit_root(
        train,
        &roots,
        tractor.beside_unit,
        tractor.along_nudge,
        tractor.side_z,
        tractor.seat_y,
    ) else {
        return Err(format!(
            "tractor.beside_unit {} out of range ({} units)",
            tractor.beside_unit,
            roots.len()
        ));
    };
    let id = format!("{}.mesh", tractor.mesh);
    let local = merge_kit_prims(mesh::extract_primitives(pack.get(&id)?)?);
    let colormap = pack.get("car.colormap")?;
    let root = kit_tr_s(
        root_pos,
        train.yaw + tractor.yaw_nudge,
        tractor.scale.max(1e-3),
    );
    let color = local.2;
    let merged = mesh::merge_transformed_prims(vec![(local, root)], color);
    Ok(Some(mesh::upload_batch(
        gpu,
        colormap,
        vec![merged],
        Mat4::IDENTITY,
        "map-a-yard-tractor",
    )?))
}

#[cfg(target_arch = "wasm32")]
fn merge_kit_prims(prims: Vec<mesh::CpuPrim>) -> mesh::CpuPrim {
    let color = prims.first().map(|p| p.2).unwrap_or([1.0, 1.0, 1.0, 1.0]);
    mesh::merge_transformed_prims(
        prims.into_iter().map(|p| (p, Mat4::IDENTITY)).collect(),
        color,
    )
}

#[cfg(target_arch = "wasm32")]
fn rail_centers(x_min: f32, x_max: f32, stride: f32) -> Vec<f32> {
    let stride = stride.max(1e-3);
    let span = (x_max - x_min).max(0.0);
    let count = (span / stride).floor() as usize;
    (0..count)
        .map(|i| x_min + (i as f32 + 0.5) * stride)
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn prim_min_y(prim: &mesh::CpuPrim) -> f32 {
    prim.0
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min)
}

/// Kit instance root: translate · rotate Y · uniform scale.
#[cfg(target_arch = "wasm32")]
fn kit_tr_s(translation: Vec3, yaw: f32, scale: f32) -> Mat4 {
    Mat4::from_translation(translation)
        * Mat4::from_rotation_y(yaw)
        * Mat4::from_scale(Vec3::splat(scale))
}

/// Structural top for seating another mesh: highest Y band below a thin top detail.
///
/// `spline-segment` has sole `0`, deck `0.05`, rail-head detail `0.075`. Using raw
/// `max_y` floats the sleeper on the detail; the deck band is the seat.
#[cfg(target_arch = "wasm32")]
fn prim_deck_y(prim: &mesh::CpuPrim) -> f32 {
    const EPS: f32 = 1e-3;
    let mut bands: Vec<f32> = Vec::new();
    for v in &prim.0 {
        let y = v.position[1];
        if !y.is_finite() {
            continue;
        }
        if !bands.iter().any(|&b| (b - y).abs() < EPS) {
            bands.push(y);
        }
    }
    bands.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    match bands.len() {
        0 => f32::NAN,
        1 => bands[0],
        2 => bands[1],
        _ => bands[bands.len() - 2],
    }
}

#[cfg(target_arch = "wasm32")]
fn sole_on_plane(plane_y: f32, min_y: f32) -> f32 {
    if min_y.is_finite() {
        plane_y - min_y
    } else {
        plane_y
    }
}

#[cfg(target_arch = "wasm32")]
fn instance_rail_atom(local: &mesh::CpuPrim, rail: &RailDef, xs: &[f32], y: f32) -> mesh::CpuPrim {
    let color = local.2;
    let scale = rail.scale.max(1e-3);
    let parts = rail
        .centerlines_z
        .iter()
        .flat_map(|&centerline_z| {
            xs.iter().map(move |&x| {
                let root = kit_tr_s(Vec3::new(x, y, centerline_z), rail.yaw, scale);
                (local.clone(), root)
            })
        })
        .collect();
    mesh::merge_transformed_prims(parts, color)
}

fn map_world_from_def(def: &MapDef) -> MapWorld {
    let train_n = def
        .train
        .as_ref()
        .map(|t| {
            t.units.len() + usize::from(t.ground_cargo.is_some()) + usize::from(t.tractor.is_some())
        })
        .unwrap_or(0);
    let mut boxes =
        Vec::with_capacity(def.shipment_containers.len() * 11 + def.boxes.len() + train_n);
    let mut ramps = Vec::with_capacity(1 + usize::from(def.train.is_some()));
    for c in &def.shipment_containers {
        boxes.extend(shipment_container_shell(
            Vec3::from_array(c.position),
            Vec3::from_array(c.half_extents),
            c.yaw,
        ));
    }
    for b in &def.boxes {
        boxes.push(MapBox {
            center: Vec3::from_array(b.position),
            half: Vec3::from_array(b.half_extents),
            yaw: 0.0,
        });
    }
    if let Some(train) = &def.train {
        let (train_boxes, train_ramps) = train_collide_solids(train);
        boxes.extend(train_boxes);
        ramps.extend(train_ramps);
    }
    if let Some(r) = &def.ramp {
        ramps.push(MapRamp {
            center_x: r.position[0],
            center_z: r.position[2],
            half_x: r.half_extents[0],
            half_z: r.half_extents[1],
            height: r.height,
            base_y: 0.0,
            yaw: r.yaw,
        });
    }
    MapWorld { boxes, ramps }
}

fn foot_surfaces_from_def(def: &MapDef) -> Result<FootSurfaces, String> {
    let mut patches = Vec::with_capacity(def.foot_patches.len() + 1);
    for p in &def.foot_patches {
        let kind = match p.kind.as_str() {
            "gravel" => FootKind::Gravel,
            "cement" => FootKind::Cement,
            "wet_cement" => FootKind::WetCement,
            "grass" => FootKind::Grass,
            "steel" => FootKind::Steel,
            other => {
                return Err(format!("map-a.def foot_patch kind: unknown {other:?}"));
            }
        };
        patches.push(FootPatch {
            kind,
            center_x: p.position[0],
            center_z: p.position[2],
            half_x: p.half_extents[0],
            half_z: p.half_extents[1],
            yaw: 0.0,
            draw: true,
        });
    }
    // Rail / train foot voices (undrawn). Later patches win.
    if let Some(rail) = &def.rail {
        let span = (rail.x_max - rail.x_min).max(0.0);
        let half_z = 0.5 * rail.scale.max(1e-3);
        let center_x = rail.x_min + span * 0.5;
        let half_x = span * 0.5;
        for &center_z in &rail.centerlines_z {
            patches.push(FootPatch {
                kind: FootKind::Cement,
                center_x,
                center_z,
                half_x,
                half_z,
                yaw: 0.0,
                draw: false,
            });
        }
    }
    if let Some(train) = &def.train {
        let boxes = train_collide_boxes(train);
        let unit_n = train.units.len();
        for (i, stem) in train.units.iter().enumerate() {
            if stem != "train-carriage-flatbed" {
                continue;
            }
            patches.push(undrawn_foot_from_box(FootKind::Steel, boxes[i]));
        }
        let mut prop_i = unit_n;
        if train.ground_cargo.is_some() {
            if let Some(&cargo_box) = boxes.get(prop_i) {
                patches.push(undrawn_foot_from_box(FootKind::Cement, cargo_box));
            }
            prop_i += 1;
        }
        if train.tractor.is_some() {
            if let Some(&tractor_box) = boxes.get(prop_i) {
                patches.push(undrawn_foot_from_box(FootKind::Steel, tractor_box));
            }
        }
    }
    for c in &def.shipment_containers {
        let shell = shipment_container_shell(
            Vec3::from_array(c.position),
            Vec3::from_array(c.half_extents),
            c.yaw,
        );
        if let Some(&floor) = shell.first() {
            patches.push(undrawn_foot_from_box(FootKind::Steel, floor));
        }
    }
    Ok(FootSurfaces { patches })
}

/// Undrawn foot voice matching a (possibly yawed) collide box — oriented, not world-AABB.
fn undrawn_foot_from_box(kind: FootKind, b: MapBox) -> FootPatch {
    FootPatch {
        kind,
        center_x: b.center.x,
        center_z: b.center.z,
        half_x: b.half.x,
        half_z: b.half.z,
        yaw: b.yaw,
        draw: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_a_rail_deserializes() {
        let json = include_str!("../../../assets/source/map-a.json");
        let def: MapDef = serde_json::from_str(json).unwrap();
        let rail = def.rail.as_ref().expect("map a has rail");
        assert_eq!(rail.centerlines_z.len(), 2);
        assert!((rail.centerlines_z[0] - (-8.0)).abs() < 1e-5);
        assert!((rail.centerlines_z[1] - (-4.8)).abs() < 1e-5);
        assert!((def.ground.half_extents[0] - 24.0).abs() < 1e-5);
        assert!((def.ground.half_extents[1] - 24.0).abs() < 1e-5);
        assert_eq!(def.shipment_containers.len(), 2);
        assert!((def.shipment_containers[0].position[0] - 8.0).abs() < 1e-5);
        assert!((def.shipment_containers[0].position[2] - 12.0).abs() < 1e-5);
        assert!((def.shipment_containers[1].position[0] - (-16.3)).abs() < 1e-5);
        assert!((def.shipment_containers[1].position[2] - (-14.0)).abs() < 1e-5);
        assert_eq!(def.shipment_containers[0].paint, "red");
        assert_eq!(def.shipment_containers[1].paint, "green");
        assert!((def.shipment_containers[1].yaw - 2.3561945).abs() < 1e-5);
        // Yard cluster stays north of the home rail; tanker-side container sits south.
        assert!(def.shipment_containers[0].position[2] > -8.0);
        for b in &def.boxes {
            assert!(b.position[2] > -8.0);
        }
        for p in &def.foot_patches {
            assert!(p.position[2] - p.half_extents[1] > -8.0);
        }
        let feet = foot_surfaces_from_def(&def).unwrap();
        assert_eq!(feet.at(18.0, -8.0), FootKind::Cement);
        assert_eq!(feet.at(18.0, -4.8), FootKind::Cement);
    }

    #[test]
    fn map_a_train_deserializes() {
        let json = include_str!("../../../assets/source/map-a.json");
        let def: MapDef = serde_json::from_str(json).unwrap();
        let train = def.train.as_ref().expect("map a has stationed train");
        assert!((train.centerline_z - (-8.0)).abs() < 1e-5);
        assert!((train.mid_x - (-2.7)).abs() < 1e-5);
        assert_eq!(
            train.units,
            [
                "train-locomotive-c",
                "train-carriage-flatbed",
                "train-carriage-flatbed",
                "train-carriage-lumber",
                "train-carriage-tank"
            ]
        );
        let cargo = train.ground_cargo.as_ref().expect("map a has ground cargo");
        assert_eq!(cargo.mesh, "lumber-cargo");
        assert_eq!(cargo.beside_unit, 1);
        let tractor = train.tractor.as_ref().expect("map a has yard tractor");
        assert_eq!(tractor.mesh, "tractor");
        assert_eq!(tractor.beside_unit, 2);
        let roots = train_unit_roots(train);
        assert_eq!(roots.len(), 5);
        // Full-length packing: tank stays west of lumber (not merged into it).
        assert!(
            roots[4].0 < roots[3].0 - 4.0,
            "tank root must stay a full car west of lumber"
        );
        let rail = def.rail.as_ref().expect("map a has rail");
        assert!((train.yaw - rail.yaw).abs() < 1e-5);
        assert!((train.centerline_z - rail.centerlines_z[0]).abs() < 1e-5);
        assert!((train.scale - 2.0).abs() < 1e-5);
        assert!((rail.scale - 2.4).abs() < 1e-5);
        assert!((rail.stride - rail.scale).abs() < 1e-5);
        assert!((train.seat_y - 0.4).abs() < 1e-5);
        assert!((train.loco_z_nudge - (-0.07)).abs() < 1e-5);
        assert!((train.unit_gap - 0.35).abs() < 1e-5);
        assert!((cargo.seat_y - (-0.55)).abs() < 1e-5);
        let world = map_world_from_def(&def);
        let train_boxes = train_collide_boxes(train);
        assert_eq!(train_boxes.len(), train.units.len() + 2);
        assert!(world.boxes.len() >= 1 + def.boxes.len() + train_boxes.len());
        let cargo_box = train_boxes[train.units.len()];
        let tractor_box = train_boxes[train.units.len() + 1];
        assert!(cargo_box.max_y() > 0.4 && cargo_box.max_y() < 0.7);
        assert!(cargo_box.min_y() < -0.4);
        let flat_top = train_boxes[1].max_y();
        let lumber_top = train_boxes[3].max_y();
        let tractor_top = tractor_box.max_y();
        assert!(flat_top > 1.0 && flat_top < 1.3);
        assert!(flat_top - cargo_box.max_y() < 1.1);
        assert!(tractor_top > flat_top && tractor_top < lumber_top);
        assert!(tractor_top - flat_top < 1.1);
        assert!(lumber_top - tractor_top < 1.1);
        assert!(tractor_box.center.z < cargo_box.center.z);
        assert!(tractor_box.center.x < cargo_box.center.x);
        assert!(cargo_box.yaw.abs() > 0.1);
        // Local half stays tight; yaw must not inflate into a world AABB.
        assert!(cargo_box.half.x < 1.1);
        assert!(cargo_box.half.z > 2.0);
        let (cmin_x, cmax_x, _cmin_z, cmax_z) = cargo_box.world_aabb_xz();
        assert!(cmax_x - cmin_x > cargo_box.half.x * 2.0 + 0.2);
        // World-AABB north tip is outside the oriented pile.
        assert!(
            (world.support_y(cargo_box.center.x, cmax_z - 0.05, f32::MAX) - cargo_box.max_y())
                .abs()
                > 0.2
        );
        // Unit boxes seal along-track except lumber→tank (nose ramp fills that joint).
        for i in 0..train.units.len() - 1 {
            if train.units[i + 1] == "train-carriage-tank" {
                continue;
            }
            let a = train_boxes[i];
            let b = train_boxes[i + 1];
            let (a_min_x, a_max_x, _, _) = a.world_aabb_xz();
            let (b_min_x, b_max_x, _, _) = b.world_aabb_xz();
            assert!(a_min_x <= b_max_x + 1e-3 && b_min_x <= a_max_x + 1e-3);
        }
        let (_boxes, train_ramps) = train_collide_solids(train);
        assert_eq!(train_ramps.len(), 2, "tank nose + rear ramps");
        let nose = train_ramps[0];
        let rear = train_ramps[1];
        assert!(nose.base_y > lumber_top && nose.base_y - lumber_top < 1.1);
        assert!((rear.base_y - nose.base_y).abs() < 1e-3);
        // Nose: yaw=consist+π → local −Z is east (lumber) = low end.
        let nose_east = nose.center_x + nose.half_z;
        let nose_mid = nose.center_x - nose.half_z;
        let nose_y = world.support_y(nose_east, train.centerline_z, nose.base_y);
        let nose_mid_y = world.support_y(nose_mid, train.centerline_z, f32::MAX);
        assert!((nose_y - nose.base_y).abs() < 0.05);
        assert!(nose_mid_y > nose_y + 0.5);
        // Rear: consist yaw → local −Z is west tip = low end.
        let rear_west = rear.center_x - rear.half_z;
        let rear_mid = rear.center_x + rear.half_z;
        let rear_y = world.support_y(rear_west, train.centerline_z, rear.base_y);
        let rear_mid_y = world.support_y(rear_mid, train.centerline_z, f32::MAX);
        assert!((rear_y - rear.base_y).abs() < 0.05);
        assert!(rear_mid_y > rear_y + 0.5);
        let feet = foot_surfaces_from_def(&def).unwrap();
        assert_eq!(
            feet.at(cargo_box.center.x, cargo_box.center.z),
            FootKind::Cement
        );
        assert_eq!(
            feet.at(tractor_box.center.x, tractor_box.center.z),
            FootKind::Steel
        );
        // World-AABB corners of a yawed tractor must not voice steel on gravel.
        let (_tmin_x, tmax_x, _tmin_z, tmax_z) = tractor_box.world_aabb_xz();
        assert_ne!(
            feet.at(tmax_x - 0.05, tmax_z - 0.05),
            FootKind::Steel,
            "tractor foot patch must be oriented, not world-AABB"
        );
        let flat = &train_boxes[1];
        assert_eq!(feet.at(flat.center.x, flat.center.z), FootKind::Steel);
    }

    #[test]
    fn default_outside_is_gravel() {
        let feet = FootSurfaces {
            patches: vec![FootPatch {
                kind: FootKind::Cement,
                center_x: 0.0,
                center_z: 0.0,
                half_x: 1.0,
                half_z: 1.0,
                yaw: 0.0,
                draw: true,
            }],
        };
        assert_eq!(feet.at(0.0, 0.0), FootKind::Cement);
        assert_eq!(feet.at(3.0, 0.0), FootKind::Gravel);
    }

    #[test]
    fn foot_kind_albedo_table() {
        assert_eq!(FootKind::Gravel.albedo(), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(FootKind::Cement.albedo(), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(FootKind::WetCement.albedo()[0], 0.42);
        assert_eq!(FootKind::Grass.albedo(), [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn map_def_requires_ground() {
        let json = r#"{
            "ground": { "position": [0.0, 0.0, 0.0], "half_extents": [12.0, 12.0] },
            "shipment_containers": [{ "position": [0.0, 1.0, 0.0], "half_extents": [1.0, 1.0, 1.0] }]
        }"#;
        let def: MapDef = serde_json::from_str(json).unwrap();
        assert_eq!(def.ground.half_extents, [12.0, 12.0]);
        assert!(def.foot_patches.is_empty());
        let world = map_world_from_def(&def);
        assert_eq!(world.boxes.len(), 11);
        let feet = foot_surfaces_from_def(&def).unwrap();
        assert_eq!(feet.at(0.0, 0.0), FootKind::Steel);
        assert_eq!(feet.at(3.0, 0.0), FootKind::Gravel);
        let floor_top = world.support_y(0.0, 0.0, 0.0);
        assert!(floor_top > 0.0 && floor_top < 0.2);
        assert!(!world.inside_solid(0.0, floor_top, 0.0));
        // Closed rear blocks; open mouth does not.
        assert!(world.inside_solid(0.0, floor_top, 0.95));
        assert!(!world.inside_solid(0.0, floor_top, -0.95));
    }

    #[test]
    fn shipment_container_shell_is_walkable_pocket() {
        let center = Vec3::new(8.0, 1.22, 12.0);
        let half = Vec3::new(1.22, 1.22, 3.04);
        let shell = shipment_container_shell(center, half, 0.0);
        assert_eq!(shell.len(), 11);
        let world = MapWorld {
            boxes: shell,
            ramps: vec![],
        };
        let floor_top = world.support_y(center.x, center.z, 0.0);
        assert!((floor_top - CONTAINER_SHELL_T).abs() < 1e-4);
        assert!((world.support_y(center.x, center.z, floor_top) - floor_top).abs() < 1e-4);
        assert!(!world.inside_solid(center.x, floor_top, center.z));
        assert!(world.inside_solid(center.x, floor_top, center.z + half.z - 0.02));
        assert!(!world.inside_solid(center.x, floor_top, center.z - half.z - 0.2));
        // Closed pose: fills frame clear, outer face flush, free edges meet at centre.
        let shut = front_leaf_pose(half, leaf_half_extents(half), -1.0, 0.0);
        let frame_inner_x = frame_inner_half_x(half);
        let front_z = -half.z - CONTAINER_FRONT_FRAME_Z;
        let frame_outer_z = front_z - CONTAINER_FRONT_FRAME_HALF_Z;
        let outer_edge_x = shut.center.x - shut.half.x;
        assert!(
            (outer_edge_x + frame_inner_x).abs() < 1e-4,
            "outer edge on post inside"
        );
        assert!(
            (shut.center.x + shut.half.x).abs() < 1e-4,
            "meets centre seal"
        );
        assert!((shut.center.z - shut.half.z - frame_outer_z).abs() < 1e-4);
        assert!((shut.half.y - (half.y - 0.11)).abs() < 1e-4);
        assert!(shut.yaw.abs() < 1e-5);
        // Rear closed leaves flush with rear frame outer face.
        let rear = rear_leaf_poses(half)[0];
        let rear_outer = half.z + CONTAINER_FRONT_FRAME_Z + CONTAINER_FRONT_FRAME_HALF_Z;
        assert!((rear.center.z + rear.half.z - rear_outer).abs() < 1e-4);
        assert!((rear.center.x - rear.half.x + frame_inner_x).abs() < 1e-4);
    }
}
