//! Self body + blaster presentation (feature 013).

use game_sim::SelfState;
use glam::Mat4;
use wasm_bindgen::JsValue;

use crate::mesh_unlit::{self, UnlitMeshGpu, UnlitMeshLayout};

pub struct SelfGpu {
    mesh: UnlitMeshGpu,
}

impl SelfGpu {
    pub async fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
        self_state: &SelfState,
    ) -> Result<Self, JsValue> {
        let layout = UnlitMeshLayout::create(device, surface_format, sample_count);
        let pack = mesh_unlit::load_kenney_core().await?;
        let gpu = layout.upload_ctx(device, queue);

        let (char_batch, blaster_batch, _, _) = mesh_unlit::upload_held_pair(
            &gpu,
            &pack,
            self_state.character,
            self_state.blaster,
            self_state.placement_matrix(),
            "self",
        )
        .map_err(|e| JsValue::from_str(&e))?;

        Ok(Self {
            mesh: layout.finish(vec![char_batch, blaster_batch]),
        })
    }

    pub fn write_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        self.mesh.write_view_proj(queue, view_proj);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.mesh.draw(pass);
    }
}

#[derive(Default)]
pub enum SelfPresentState {
    #[default]
    Idle,
    Loading,
    Ready(SelfGpu),
    #[allow(dead_code)]
    Failed(String),
}
