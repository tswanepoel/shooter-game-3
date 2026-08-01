//! Debug blaster lineup: each blaster held by a Kenney character,
//! with magenta balls at every muzzle point.
//! Held attach and muzzle world: `held_blaster · muzzle_local`.
//! Kit facts: `assets/source/characters/README.md`, `assets/source/blasters/README.md`.
//! Loads via cook pack `kenney-core`, not source paths.

use glam::{Mat4, Vec3};
use wasm_bindgen::JsValue;

use crate::mesh::{self, UnlitMeshGpu, UnlitMeshLayout};

const LETTERS: &[u8] = b"abcdefghijklmnopqr";

/// Row spacing (m); wider than character-only so held weapons clear neighbours.
const LINEUP_SPACING_M: f32 = 2.0;
/// Row depth (m); stub cam looks −Z historically; lineup sits in −Z for flycam inspect.
const LINEUP_Z_M: f32 = -6.0;

/// Magenta muzzle marker radius in world metres (feature 012).
const MUZZLE_MARKER_RADIUS_M: f32 = 0.0175;
/// Solid magenta (sRGB factors for unlit path).
const MUZZLE_MARKER_COLOR: [f32; 4] = [1.0, 0.0, 1.0, 1.0];

pub struct LineupGpu {
    mesh: UnlitMeshGpu,
}

impl LineupGpu {
    pub async fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Result<Self, JsValue> {
        let layout = UnlitMeshLayout::create(device, surface_format, sample_count);
        let pack = mesh::load_kenney_core().await?;
        let gpu = layout.upload_ctx(device, queue);

        let n = LETTERS.len();
        let mut batches = Vec::with_capacity(n * 4);

        for (i, &letter) in LETTERS.iter().enumerate() {
            let x = (i as f32 - (n as f32 - 1.0) * 0.5) * LINEUP_SPACING_M;
            let placement = Mat4::from_translation(Vec3::new(x, 0.0, LINEUP_Z_M));

            let (char_batch, blaster_batch, held_blaster) =
                mesh::upload_held_pair(&gpu, &pack, letter, letter, placement, "lineup")
                    .map_err(|e| JsValue::from_str(&format!("lineup {}: {e}", letter as char)))?;
            batches.push(char_batch);
            batches.push(blaster_batch);

            // Magenta balls at every muzzle (012): blaster-local under held root (037).
            for muzzle_world in mesh::muzzle_world_points(held_blaster, i) {
                let marker_root = Mat4::from_translation(muzzle_world)
                    * Mat4::from_scale(Vec3::splat(MUZZLE_MARKER_RADIUS_M));
                let marker_batch = mesh::upload_solid_batch(
                    &gpu,
                    mesh::unit_sphere_prim(12, 8),
                    marker_root,
                    MUZZLE_MARKER_COLOR,
                    mesh::SolidShading::Unlit,
                    "lineup-muzzle-marker",
                )
                .map_err(|e| {
                    JsValue::from_str(&format!("muzzle marker {}: {e}", letter as char))
                })?;
                batches.push(marker_batch);
            }
        }

        Ok(Self {
            mesh: layout.finish(batches),
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
pub enum LineupState {
    #[default]
    Idle,
    Loading,
    Ready(LineupGpu),
    Failed,
}
