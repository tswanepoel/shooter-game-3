//! Cooked map load and draw (064 landmark, 066 solids, 070/072/073 foot patches, 082 ground, 083 gravel, 084 cement, 085 grass).

#[cfg(target_arch = "wasm32")]
use glam::Mat4;
use glam::Vec3;
use serde::Deserialize;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(target_arch = "wasm32")]
use crate::mesh::{self, UnlitMeshGpu, UnlitMeshLayout};
#[cfg(target_arch = "wasm32")]
use crate::pack;
use game_sim::{MapBox, MapRamp, MapWorld};

pub const MAP_A_PACK: &str = "maps-a";

const LANDMARK_COLOR: [f32; 4] = [0.55, 0.62, 0.72, 1.0];
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
    landmark: LandmarkDef,
    #[serde(default)]
    boxes: Vec<LandmarkDef>,
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
struct LandmarkDef {
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
                            &label,
                        )
                        .map_err(|e| JsValue::from_str(&e))?
                    }
                }
            } else {
                mesh::upload_solid_batch(&gpu, mesh::box_prim(half, color), root, color, &label)
                    .map_err(|e| JsValue::from_str(&e))?
            };
            batches.push(batch);
        }

        let half = Vec3::from_array(def.landmark.half_extents);
        let root = Mat4::from_translation(Vec3::from_array(def.landmark.position));
        batches.push(
            mesh::upload_solid_batch(
                &gpu,
                mesh::box_prim(half, LANDMARK_COLOR),
                root,
                LANDMARK_COLOR,
                "map-a-landmark",
            )
            .map_err(|e| JsValue::from_str(&e))?,
        );

        for (i, b) in def.boxes.iter().enumerate() {
            let half = Vec3::from_array(b.half_extents);
            let root = Mat4::from_translation(Vec3::from_array(b.position));
            batches.push(
                mesh::upload_solid_batch(
                    &gpu,
                    mesh::box_prim(half, BOX_COLOR),
                    root,
                    BOX_COLOR,
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

    pub fn write_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        self.mesh.write_view_proj(queue, view_proj);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.mesh.draw(pass);
    }
}

fn map_world_from_def(def: &MapDef) -> MapWorld {
    let mut boxes = Vec::with_capacity(1 + def.boxes.len());
    boxes.push(MapBox {
        center: Vec3::from_array(def.landmark.position),
        half: Vec3::from_array(def.landmark.half_extents),
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
            "landmark": { "position": [0.0, 1.0, 0.0], "half_extents": [1.0, 1.0, 1.0] }
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
