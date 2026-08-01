//! Cooked map load and draw (064 shipment container, 066 solids, 070/072/073 foot patches, 082 ground, 083 gravel, 084 cement, 085 grass, 086 container albedo, 087 closed door hardware, 088 lit map solids, 089 morning light, 090 rail corridor, 091 stationed train).

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
const CONTAINER_FRAME_COLOR: [f32; 4] = [0.39, 0.17, 0.14, 1.0];
const CONTAINER_GASKET_COLOR: [f32; 4] = [0.025, 0.028, 0.026, 1.0];
const CONTAINER_HARDWARE_COLOR: [f32; 4] = [0.42, 0.44, 0.41, 1.0];

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

    for z_sign in [1.0_f32, -1.0] {
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
        let gasket_half_thickness = 0.018;
        let gasket_edge_x = half.x - 0.035;
        let gasket_edge_y = half.y - 0.115;
        gasket.push(cuboid(
            Vec3::new(0.0, gasket_edge_y, gasket_z),
            Vec3::new(
                gasket_edge_x + gasket_half_thickness,
                gasket_half_thickness,
                0.014,
            ),
            CONTAINER_GASKET_COLOR,
        ));
        gasket.push(cuboid(
            Vec3::new(0.0, -gasket_edge_y, gasket_z),
            Vec3::new(
                gasket_edge_x + gasket_half_thickness,
                gasket_half_thickness,
                0.014,
            ),
            CONTAINER_GASKET_COLOR,
        ));
        gasket.push(cuboid(
            Vec3::new(-gasket_edge_x, 0.0, gasket_z),
            Vec3::new(
                gasket_half_thickness,
                gasket_edge_y + gasket_half_thickness,
                0.014,
            ),
            CONTAINER_GASKET_COLOR,
        ));
        gasket.push(cuboid(
            Vec3::new(gasket_edge_x, 0.0, gasket_z),
            Vec3::new(
                gasket_half_thickness,
                gasket_edge_y + gasket_half_thickness,
                0.014,
            ),
            CONTAINER_GASKET_COLOR,
        ));
        gasket.push(cuboid(
            Vec3::new(0.0, 0.0, out(0.088)),
            Vec3::new(
                gasket_half_thickness,
                gasket_edge_y + gasket_half_thickness,
                0.014,
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

        for sign in [-1.0_f32, 1.0] {
            for y in [-half.y * 0.72, -half.y * 0.24, half.y * 0.24, half.y * 0.72] {
                frame.push(cuboid(
                    Vec3::new(sign * (half.x - 0.055), y, out(0.105)),
                    Vec3::new(0.065, 0.045, 0.025),
                    CONTAINER_FRAME_COLOR,
                ));
                frame.push(cylinder(
                    Vec3::new(sign * (half.x - 0.010), y, out(0.135)),
                    0.025,
                    0.075,
                    Mat4::IDENTITY,
                    CONTAINER_FRAME_COLOR,
                ));
            }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootKind {
    Gravel,
    Cement,
    WetCement,
    Grass,
}

impl FootKind {
    /// Shared kind → albedo for ground and foot pads (082).
    pub fn albedo(self) -> [f32; 4] {
        match self {
            Self::Gravel => [1.0, 1.0, 1.0, 1.0],
            Self::Cement => [1.0, 1.0, 1.0, 1.0],
            Self::WetCement => [0.42, 0.48, 0.52, 1.0],
            Self::Grass => [1.0, 1.0, 1.0, 1.0],
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
}

impl FootPatch {
    fn contains(self, x: f32, z: f32) -> bool {
        (x - self.center_x).abs() <= self.half_x && (z - self.center_z).abs() <= self.half_z
    }
}

/// Present-only foot surface patches (`gravel` / `cement` / `wet_cement` / `grass`). Outside → gravel.
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
    shipment_container: SolidBoxDef,
    #[serde(default)]
    boxes: Vec<SolidBoxDef>,
    #[serde(default)]
    ramp: Option<RampDef>,
    #[serde(default)]
    foot_patches: Vec<FootPatchDef>,
}

/// Straight east–west rail corridor (090). Present only — no collide.
#[derive(Debug, Deserialize)]
struct RailDef {
    centerline_z: f32,
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

/// Parked freight consist on the rail corridor (091). Present only — no collide.
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
}

/// Present-only ground pile next to the consist (091).
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

        let half = Vec3::from_array(def.shipment_container.half_extents);
        let root = Mat4::from_translation(Vec3::from_array(def.shipment_container.position));
        let container_tint = [1.0, 1.0, 1.0, 1.0];
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
        let sides = upload_container_part(
            "container-side.albedo",
            mesh::BoxFaceGroup::Sides,
            side_metres_per_tile,
            SolidUvLayout::BoxFace,
            "map-a-shipment-container-sides",
        );
        let front = upload_container_part(
            "container-door.albedo",
            mesh::BoxFaceGroup::Front,
            1.0,
            SolidUvLayout::RearDoors,
            "map-a-shipment-container-front",
        );
        let lids = upload_container_part(
            "container-side.albedo",
            mesh::BoxFaceGroup::Lids,
            side_metres_per_tile,
            SolidUvLayout::BoxFace,
            "map-a-shipment-container-lids",
        );
        let rear = upload_container_part(
            "container-door.albedo",
            mesh::BoxFaceGroup::Rear,
            1.0,
            SolidUvLayout::RearDoors,
            "map-a-shipment-container-rear",
        );
        match (sides, front, lids, rear) {
            (Ok(sides), Ok(front), Ok(lids), Ok(rear)) => {
                batches.push(sides);
                batches.push(front);
                batches.push(lids);
                batches.push(rear);
            }
            (s, f, l, r) => {
                let err = s
                    .err()
                    .or(f.err())
                    .or(l.err())
                    .or(r.err())
                    .unwrap_or_else(|| "container albedo".into());
                web_sys::console::warn_1(
                    &format!("map: container albedo unusable ({err}); flat container").into(),
                );
                batches.push(
                    mesh::upload_solid_batch(
                        &gpu,
                        mesh::box_prim(half, CONTAINER_FALLBACK_COLOR),
                        root,
                        CONTAINER_FALLBACK_COLOR,
                        mesh::SolidShading::Lit,
                        "map-a-shipment-container",
                    )
                    .map_err(|e| JsValue::from_str(&e))?,
                );
            }
        }
        for (prim, color, label) in container_door_hardware(half) {
            batches.push(
                mesh::upload_solid_batch(&gpu, prim, root, color, mesh::SolidShading::Lit, label)
                    .map_err(|e| JsValue::from_str(&e))?,
            );
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

/// Tile Kenney spline atoms along the rail corridor and upload two lit batches (090).
#[cfg(target_arch = "wasm32")]
fn upload_rail_corridor(
    gpu: &mesh::UploadCtx<'_>,
    pack: &pack::Pack,
    rail: &RailDef,
) -> Result<Vec<mesh::MeshBatch>, String> {
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

    let gap = train.unit_gap.max(0.0);
    let mut total = gap * train.units.len().saturating_sub(1) as f32;
    let mut extents = Vec::with_capacity(piece_locals.len());
    for local in &piece_locals {
        let (min_z, max_z) = prim_z_extent(local);
        if !min_z.is_finite() || !max_z.is_finite() || max_z <= min_z {
            return Err("train piece missing along-track extent".into());
        }
        total += (max_z - min_z) * scale;
        extents.push((min_z, max_z));
    }

    let mut front_tip = train.mid_x + total * 0.5;
    let mut parts = Vec::with_capacity(piece_locals.len() + 1);
    let mut unit_centers_x = Vec::with_capacity(piece_locals.len());
    for (i, ((stem, local), (min_z, max_z))) in train
        .units
        .iter()
        .zip(piece_locals.iter())
        .zip(extents.iter().copied())
        .enumerate()
    {
        let x = front_tip - max_z * scale;
        let z = if stem == "train-locomotive-c" {
            train.centerline_z + train.loco_z_nudge
        } else {
            train.centerline_z
        };
        let root = kit_tr_s(Vec3::new(x, train.seat_y, z), train.yaw, scale);
        parts.push((local.clone(), root));
        unit_centers_x.push(x);
        front_tip = x + min_z * scale;
        if i + 1 < train.units.len() {
            front_tip -= gap;
        }
    }

    if let Some(cargo) = &train.ground_cargo {
        let i = cargo.beside_unit;
        if i >= unit_centers_x.len() {
            return Err(format!(
                "ground_cargo.beside_unit {i} out of range ({} units)",
                unit_centers_x.len()
            ));
        }
        let id = format!("{}.mesh", cargo.mesh);
        let local = merge_kit_prims(mesh::extract_primitives(pack.get(&id)?)?);
        let cargo_scale = cargo.scale.max(1e-3);
        let root = kit_tr_s(
            Vec3::new(
                unit_centers_x[i] + cargo.along_nudge,
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

#[cfg(target_arch = "wasm32")]
fn prim_z_extent(prim: &mesh::CpuPrim) -> (f32, f32) {
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for v in &prim.0 {
        min_z = min_z.min(v.position[2]);
        max_z = max_z.max(v.position[2]);
    }
    (min_z, max_z)
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
    let parts = xs
        .iter()
        .map(|&x| {
            let root = kit_tr_s(Vec3::new(x, y, rail.centerline_z), rail.yaw, scale);
            (local.clone(), root)
        })
        .collect();
    mesh::merge_transformed_prims(parts, color)
}

fn map_world_from_def(def: &MapDef) -> MapWorld {
    let mut boxes = Vec::with_capacity(1 + def.boxes.len());
    boxes.push(MapBox {
        center: Vec3::from_array(def.shipment_container.position),
        half: Vec3::from_array(def.shipment_container.half_extents),
    });
    for b in &def.boxes {
        boxes.push(MapBox {
            center: Vec3::from_array(b.position),
            half: Vec3::from_array(b.half_extents),
        });
    }
    let ramps = def
        .ramp
        .as_ref()
        .map(|r| {
            vec![MapRamp {
                center_x: r.position[0],
                center_z: r.position[2],
                half_x: r.half_extents[0],
                half_z: r.half_extents[1],
                height: r.height,
                yaw: r.yaw,
            }]
        })
        .unwrap_or_default();
    MapWorld { boxes, ramps }
}

fn foot_surfaces_from_def(def: &MapDef) -> Result<FootSurfaces, String> {
    let mut patches = Vec::with_capacity(def.foot_patches.len());
    for p in &def.foot_patches {
        let kind = match p.kind.as_str() {
            "gravel" => FootKind::Gravel,
            "cement" => FootKind::Cement,
            "wet_cement" => FootKind::WetCement,
            "grass" => FootKind::Grass,
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
        });
    }
    Ok(FootSurfaces { patches })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_a_rail_deserializes() {
        let json = include_str!("../../../assets/source/map-a.json");
        let def: MapDef = serde_json::from_str(json).unwrap();
        let rail = def.rail.expect("map a has rail");
        assert!((rail.centerline_z - (-8.0)).abs() < 1e-5);
        assert!((def.ground.half_extents[0] - 24.0).abs() < 1e-5);
        assert!((def.ground.half_extents[1] - 24.0).abs() < 1e-5);
        assert!(def.shipment_container.position[2] > -8.0);
        for b in &def.boxes {
            assert!(b.position[2] > -8.0);
        }
        for p in &def.foot_patches {
            assert!(p.position[2] - p.half_extents[1] > -8.0);
        }
    }

    #[test]
    fn map_a_train_deserializes() {
        let json = include_str!("../../../assets/source/map-a.json");
        let def: MapDef = serde_json::from_str(json).unwrap();
        let train = def.train.expect("map a has stationed train");
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
        let cargo = train.ground_cargo.expect("map a has ground cargo");
        assert_eq!(cargo.mesh, "lumber-cargo");
        assert_eq!(cargo.beside_unit, 2);
        let rail = def.rail.expect("map a has rail");
        assert!((train.yaw - rail.yaw).abs() < 1e-5);
        assert!((train.centerline_z - rail.centerline_z).abs() < 1e-5);
        assert!((train.scale - 2.0).abs() < 1e-5);
        assert!((rail.scale - 2.4).abs() < 1e-5);
        assert!((rail.stride - rail.scale).abs() < 1e-5);
        assert!((train.seat_y - 0.4).abs() < 1e-5);
        assert!((train.loco_z_nudge - (-0.07)).abs() < 1e-5);
        assert!((train.unit_gap - 0.35).abs() < 1e-5);
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
            "shipment_container": { "position": [0.0, 1.0, 0.0], "half_extents": [1.0, 1.0, 1.0] }
        }"#;
        let def: MapDef = serde_json::from_str(json).unwrap();
        assert_eq!(def.ground.half_extents, [12.0, 12.0]);
        assert!(def.foot_patches.is_empty());
        let world = map_world_from_def(&def);
        assert_eq!(world.boxes.len(), 1);
        let feet = foot_surfaces_from_def(&def).unwrap();
        assert_eq!(feet.at(0.0, 0.0), FootKind::Gravel);
    }
}
