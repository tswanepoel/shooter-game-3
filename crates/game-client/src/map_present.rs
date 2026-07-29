//! Cooked map load and draw (064 landmark, 066 solids).

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

#[derive(Debug, Deserialize)]
struct MapDef {
    landmark: LandmarkDef,
    #[serde(default)]
    boxes: Vec<LandmarkDef>,
    #[serde(default)]
    ramp: Option<RampDef>,
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
    ) -> Result<(Self, MapWorld), JsValue> {
        let pack = pack::load_pack(MAP_A_PACK).await?;
        let def_bytes = pack.get("map-a.def").map_err(|e| JsValue::from_str(&e))?;
        let def: MapDef = serde_json::from_slice(def_bytes)
            .map_err(|e| JsValue::from_str(&format!("map-a.def: {e}")))?;

        let world = map_world_from_def(&def);

        let layout = UnlitMeshLayout::create(device, surface_format, sample_count);
        let gpu = layout.upload_ctx(device, queue);
        let mut batches = Vec::new();

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
