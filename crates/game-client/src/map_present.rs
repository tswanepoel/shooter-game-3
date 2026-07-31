//! Cooked map load and draw (064 landmark, 066 solids, 070/072/073 foot patches).

use glam::{Mat4, Vec3};
use serde::Deserialize;
use wasm_bindgen::JsValue;

use crate::mesh::{self, UnlitMeshGpu, UnlitMeshLayout};
use crate::pack;
use game_sim::{MapBox, MapRamp, MapWorld};

pub const MAP_A_PACK: &str = "maps-a";

const LANDMARK_COLOR: [f32; 4] = [0.55, 0.62, 0.72, 1.0];
const BOX_COLOR: [f32; 4] = [0.72, 0.58, 0.42, 1.0];
const RAMP_COLOR: [f32; 4] = [0.48, 0.66, 0.52, 1.0];
const CEMENT_PATCH_COLOR: [f32; 4] = [0.58, 0.58, 0.6, 1.0];
const WET_CEMENT_PATCH_COLOR: [f32; 4] = [0.42, 0.48, 0.52, 1.0];
const GRAVEL_PATCH_COLOR: [f32; 4] = [0.55, 0.48, 0.38, 1.0];
const GRASS_PATCH_COLOR: [f32; 4] = [0.42, 0.58, 0.36, 1.0];
/// Visual slab half-height for foot patches (present only — not collide).
const FOOT_PATCH_HALF_Y: f32 = 0.02;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootKind {
    Gravel,
    Cement,
    WetCement,
    Grass,
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
    landmark: LandmarkDef,
    #[serde(default)]
    boxes: Vec<LandmarkDef>,
    #[serde(default)]
    ramp: Option<RampDef>,
    #[serde(default)]
    foot_patches: Vec<FootPatchDef>,
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

pub struct MapGpu {
    mesh: UnlitMeshGpu,
}

pub enum MapPresentState {
    Idle,
    Loading,
    Ready(MapGpu),
    Failed,
}

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
        let feet = foot_surfaces_from_def(&def)?;

        let layout = UnlitMeshLayout::create(device, surface_format, sample_count);
        let gpu = layout.upload_ctx(device, queue);
        let mut batches = Vec::new();

        for (i, p) in feet.patches.iter().enumerate() {
            let color = match p.kind {
                FootKind::Cement => CEMENT_PATCH_COLOR,
                FootKind::WetCement => WET_CEMENT_PATCH_COLOR,
                FootKind::Gravel => GRAVEL_PATCH_COLOR,
                FootKind::Grass => GRASS_PATCH_COLOR,
            };
            let half = Vec3::new(p.half_x, FOOT_PATCH_HALF_Y, p.half_z);
            let root = Mat4::from_translation(Vec3::new(p.center_x, FOOT_PATCH_HALF_Y, p.center_z));
            batches.push(
                mesh::upload_solid_batch(
                    &gpu,
                    mesh::box_prim(half, color),
                    root,
                    color,
                    &format!("map-a-foot-{i}"),
                )
                .map_err(|e| JsValue::from_str(&e))?,
            );
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

fn foot_surfaces_from_def(def: &MapDef) -> Result<FootSurfaces, JsValue> {
    let mut patches = Vec::with_capacity(def.foot_patches.len());
    for p in &def.foot_patches {
        let kind = match p.kind.as_str() {
            "gravel" => FootKind::Gravel,
            "cement" => FootKind::Cement,
            "wet_cement" => FootKind::WetCement,
            "grass" => FootKind::Grass,
            other => {
                return Err(JsValue::from_str(&format!(
                    "map-a.def foot_patch kind: unknown {other:?}"
                )));
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
}
