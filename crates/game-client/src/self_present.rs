//! Self body + blaster presentation (013/015/016).

use game_sim::{SelfState, FACE_OFFSET_HEAD_KIT};
use glam::{Mat4, Vec3};
use wasm_bindgen::JsValue;

use crate::mesh_unlit::{self, AnimClip, CharPart, MeshVertex, UnlitMeshGpu, UnlitMeshLayout};

pub struct MountedView {
    pub eye: Vec3,
    pub reticle_world: Option<Vec3>,
}

pub struct SelfGpu {
    mesh: UnlitMeshGpu,
    parts: Vec<CharPart>,
    /// Kenney `walk` clip (phase from sim).
    walk_clip: AnimClip,
    /// part index → primitive index in character batch (0), if meshful.
    part_prim: Vec<Option<usize>>,
    blaster_local: Vec<Vec<MeshVertex>>,
    blaster_batch: usize,
    min_y: f32,
    letter_index: usize,
    pub view: MountedView,
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

        let ch = self_state.character as char;
        let bl = self_state.blaster as char;
        let bi = mesh_unlit::letter_index(self_state.blaster).map_err(|e| JsValue::from_str(&e))?;

        let char_glb = pack
            .get(&format!("character-{ch}.mesh"))
            .map_err(|e| JsValue::from_str(&e))?;
        let char_png = pack
            .get(&format!("character-{ch}.albedo"))
            .map_err(|e| JsValue::from_str(&e))?;
        let blaster_glb = pack
            .get(&format!("blaster-{bl}.mesh"))
            .map_err(|e| JsValue::from_str(&e))?;
        let colormap = pack
            .get("blaster.colormap")
            .map_err(|e| JsValue::from_str(&e))?;

        let (parts, min_y) =
            mesh_unlit::extract_character_parts(char_glb).map_err(|e| JsValue::from_str(&e))?;
        let walk_clip =
            mesh_unlit::extract_clip(char_glb, "walk").map_err(|e| JsValue::from_str(&e))?;
        let blaster_prims =
            mesh_unlit::extract_primitives(blaster_glb).map_err(|e| JsValue::from_str(&e))?;

        let (worlds, arm_kit) =
            mesh_unlit::pose_character_kit(&parts, self_state, Some(&walk_clip));
        let k2w = mesh_unlit::kit_to_world(self_state.placement_matrix(), min_y);

        let mut char_cpu: Vec<(Vec<MeshVertex>, Vec<u32>, [f32; 4])> = Vec::new();
        let mut part_prim = vec![None; parts.len()];
        for (i, part) in parts.iter().enumerate() {
            if part.local_verts.is_empty() {
                continue;
            }
            let world = k2w * worlds[i];
            let mut verts = part.local_verts.clone();
            for v in &mut verts {
                v.position = world
                    .transform_point3(Vec3::from_array(v.position))
                    .to_array();
            }
            part_prim[i] = Some(char_cpu.len());
            char_cpu.push((verts, part.indices.clone(), part.base_color));
        }

        let aim_pitch = self_state.torso_pitch + self_state.shoulder_pitch;
        let blaster_root = mesh_unlit::held_blaster_root(k2w, arm_kit, bi, aim_pitch);
        let mut blaster_local = Vec::new();
        let mut blaster_cpu = Vec::new();
        for (verts, indices, color) in blaster_prims {
            blaster_local.push(verts.clone());
            let mut world_verts = verts;
            for v in &mut world_verts {
                v.position = blaster_root
                    .transform_point3(Vec3::from_array(v.position))
                    .to_array();
            }
            blaster_cpu.push((world_verts, indices, color));
        }

        let char_batch =
            mesh_unlit::upload_batch(&gpu, char_png, char_cpu, Mat4::IDENTITY, "self-character")
                .map_err(|e| JsValue::from_str(&e))?;
        let blaster_batch =
            mesh_unlit::upload_batch(&gpu, colormap, blaster_cpu, Mat4::IDENTITY, "self-blaster")
                .map_err(|e| JsValue::from_str(&e))?;

        let mut s = Self {
            mesh: layout.finish(vec![char_batch, blaster_batch]),
            parts,
            walk_clip,
            part_prim,
            blaster_local,
            blaster_batch: 1,
            min_y,
            letter_index: bi,
            view: MountedView {
                eye: Vec3::ZERO,
                reticle_world: None,
            },
        };
        s.apply_state(queue, self_state);
        Ok(s)
    }

    pub fn apply_state(&mut self, queue: &wgpu::Queue, self_state: &SelfState) {
        let (worlds, arm_kit) =
            mesh_unlit::pose_character_kit(&self.parts, self_state, Some(&self.walk_clip));
        let k2w = mesh_unlit::kit_to_world(self_state.placement_matrix(), self.min_y);

        for (i, part) in self.parts.iter().enumerate() {
            let Some(prim) = self.part_prim[i] else {
                continue;
            };
            let world = k2w * worlds[i];
            let mut verts = part.local_verts.clone();
            for v in &mut verts {
                v.position = world
                    .transform_point3(Vec3::from_array(v.position))
                    .to_array();
            }
            self.mesh.write_prim_verts(queue, 0, prim, &verts);
        }

        let aim_pitch = self_state.torso_pitch + self_state.shoulder_pitch;
        let blaster_root =
            mesh_unlit::held_blaster_root(k2w, arm_kit, self.letter_index, aim_pitch);
        for (pi, local) in self.blaster_local.iter().enumerate() {
            let mut verts = local.clone();
            for v in &mut verts {
                v.position = blaster_root
                    .transform_point3(Vec3::from_array(v.position))
                    .to_array();
            }
            self.mesh
                .write_prim_verts(queue, self.blaster_batch, pi, &verts);
        }

        let head_kit = self
            .parts
            .iter()
            .position(|p| p.name == "head")
            .map(|i| worlds[i])
            .unwrap_or(Mat4::IDENTITY);
        let eye = k2w.transform_point3(head_kit.transform_point3(FACE_OFFSET_HEAD_KIT));
        let reticle_world = self_state.reticle_world(eye);

        self.view = MountedView { eye, reticle_world };
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
