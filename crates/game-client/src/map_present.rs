//! Cooked map load and draw (064 map-a landmark).

use glam::{Mat4, Vec3};
use serde::Deserialize;
use wasm_bindgen::JsValue;

use crate::mesh::{self, UnlitMeshGpu, UnlitMeshLayout};
use crate::pack;

pub const MAP_A_PACK: &str = "maps-a";

const LANDMARK_COLOR: [f32; 4] = [0.55, 0.62, 0.72, 1.0];

#[derive(Debug, Deserialize)]
struct MapDef {
    landmark: LandmarkDef,
}

#[derive(Debug, Deserialize)]
struct LandmarkDef {
    position: [f32; 3],
    half_extents: [f32; 3],
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
    ) -> Result<Self, JsValue> {
        let pack = pack::load_pack(MAP_A_PACK).await?;
        let def_bytes = pack.get("map-a.def").map_err(|e| JsValue::from_str(&e))?;
        let def: MapDef = serde_json::from_slice(def_bytes)
            .map_err(|e| JsValue::from_str(&format!("map-a.def: {e}")))?;

        let layout = UnlitMeshLayout::create(device, surface_format, sample_count);
        let gpu = layout.upload_ctx(device, queue);
        let half = Vec3::from_array(def.landmark.half_extents);
        let root = Mat4::from_translation(Vec3::from_array(def.landmark.position));
        let batch = mesh::upload_solid_batch(
            &gpu,
            mesh::box_prim(half, LANDMARK_COLOR),
            root,
            LANDMARK_COLOR,
            "map-a-landmark",
        )
        .map_err(|e| JsValue::from_str(&e))?;

        Ok(Self {
            mesh: layout.finish(vec![batch]),
        })
    }

    pub fn write_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        self.mesh.write_view_proj(queue, view_proj);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.mesh.draw(pass);
    }
}
