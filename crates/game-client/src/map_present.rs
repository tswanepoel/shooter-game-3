//! Cooked map load and draw (064 shipment container, 066 solids, 070/072/073 foot patches, 082 ground, 083 gravel, 084 cement, 085 grass, 086 container albedo, 087 closed door hardware, 088 lit map solids, 089 morning light).

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
    shipment_container: SolidBoxDef,
    #[serde(default)]
    boxes: Vec<SolidBoxDef>,
    #[serde(default)]
    ramp: Option<RampDef>,
    #[serde(default)]
    foot_patches: Vec<FootPatchDef>,
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
