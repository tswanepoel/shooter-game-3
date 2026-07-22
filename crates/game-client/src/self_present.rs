//! Self body + blaster presentation (013/015/016/017/021).
//!
//! Present pose draws the body (walk included). Look pose mounts the view (017).
//! Loadout may hold primary + secondary meshes; only the active letter is shown.

use game_sim::{SelfState, FACE_OFFSET_HEAD_KIT};
use glam::{Mat4, Vec3};
use wasm_bindgen::JsValue;

use crate::mesh_unlit::{
    self, AnimClip, CharPart, KitPose, MeshVertex, UnlitMeshGpu, UnlitMeshLayout,
};

/// Mounted first-person mount from the look pose (017).
pub struct MountedView {
    /// Face point on the look pose — view and aim start here.
    pub look_origin: Vec3,
    pub reticle_world: Option<Vec3>,
}

struct EquippedBlaster {
    letter: u8,
    letter_index: usize,
    local: Vec<Vec<MeshVertex>>,
    batch: usize,
}

pub struct SelfGpu {
    mesh: UnlitMeshGpu,
    parts: Vec<CharPart>,
    /// Kenney `walk` clip (phase from sim).
    walk_clip: AnimClip,
    /// Kenney `sprint` clip (phase from sim while sprinting).
    sprint_clip: AnimClip,
    /// part index → primitive index in character batch (0), if meshful.
    part_prim: Vec<Option<usize>>,
    blasters: Vec<EquippedBlaster>,
    min_y: f32,
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

        let char_glb = pack
            .get(&format!("character-{ch}.mesh"))
            .map_err(|e| JsValue::from_str(&e))?;
        let char_png = pack
            .get(&format!("character-{ch}.albedo"))
            .map_err(|e| JsValue::from_str(&e))?;

        let (parts, min_y) =
            mesh_unlit::extract_character_parts(char_glb).map_err(|e| JsValue::from_str(&e))?;
        let walk_clip =
            mesh_unlit::extract_clip(char_glb, "walk").map_err(|e| JsValue::from_str(&e))?;
        let sprint_clip =
            mesh_unlit::extract_clip(char_glb, "sprint").map_err(|e| JsValue::from_str(&e))?;

        let loco = if self_state.locomotion.is_sprint() {
            &sprint_clip
        } else {
            &walk_clip
        };
        let (worlds, arm_kit) =
            mesh_unlit::pose_character_kit(&parts, self_state, Some(loco), KitPose::Present);
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
                mesh_unlit::transform_vertex(v, world);
            }
            part_prim[i] = Some(char_cpu.len());
            char_cpu.push((verts, part.indices.clone(), part.base_color));
        }

        let char_batch =
            mesh_unlit::upload_batch(&gpu, char_png, char_cpu, Mat4::IDENTITY, "self-character")
                .map_err(|e| JsValue::from_str(&e))?;

        let mut batches = vec![char_batch];
        let mut blasters = Vec::new();
        let mut seen = Vec::new();
        for letter in [self_state.primary, self_state.secondary]
            .into_iter()
            .flatten()
        {
            if seen.contains(&letter) {
                continue;
            }
            seen.push(letter);
            let bi = mesh_unlit::letter_index(letter).map_err(|e| JsValue::from_str(&e))?;
            let bl = letter as char;
            let blaster_glb = pack
                .get(&format!("blaster-{bl}.mesh"))
                .map_err(|e| JsValue::from_str(&e))?;
            let colormap = pack
                .get("blaster.colormap")
                .map_err(|e| JsValue::from_str(&e))?;
            let blaster_prims =
                mesh_unlit::extract_primitives(blaster_glb).map_err(|e| JsValue::from_str(&e))?;

            let show = self_state.active_blaster() == Some(letter);
            let root = if show {
                mesh_unlit::held_blaster_root(k2w, arm_kit, bi)
            } else {
                Mat4::from_scale(Vec3::ZERO)
            };

            let mut local = Vec::new();
            let mut cpu = Vec::new();
            for (verts, indices, color) in blaster_prims {
                local.push(verts.clone());
                let mut world_verts = verts;
                for v in &mut world_verts {
                    mesh_unlit::transform_vertex(v, root);
                }
                cpu.push((world_verts, indices, color));
            }
            let batch_idx = batches.len();
            let batch = mesh_unlit::upload_batch(
                &gpu,
                colormap,
                cpu,
                Mat4::IDENTITY,
                &format!("self-blaster-{bl}"),
            )
            .map_err(|e| JsValue::from_str(&e))?;
            batches.push(batch);
            blasters.push(EquippedBlaster {
                letter,
                letter_index: bi,
                local,
                batch: batch_idx,
            });
        }

        let mut s = Self {
            mesh: layout.finish(batches),
            parts,
            walk_clip,
            sprint_clip,
            part_prim,
            blasters,
            min_y,
            view: MountedView {
                look_origin: Vec3::ZERO,
                reticle_world: None,
            },
        };
        s.apply_state(queue, self_state);
        Ok(s)
    }

    /// Body + active blaster from drive (walk/sprint/jump/stand).
    pub fn apply_present(&mut self, queue: &wgpu::Queue, self_state: &SelfState) {
        let k2w = mesh_unlit::kit_to_world(self_state.placement_matrix(), self.min_y);

        let loco = if self_state.locomotion.is_sprint() {
            &self.sprint_clip
        } else {
            &self.walk_clip
        };
        let (present_worlds, arm_kit) =
            mesh_unlit::pose_character_kit(&self.parts, self_state, Some(loco), KitPose::Present);

        for (i, part) in self.parts.iter().enumerate() {
            let Some(prim) = self.part_prim[i] else {
                continue;
            };
            let world = k2w * present_worlds[i];
            let mut verts = part.local_verts.clone();
            for v in &mut verts {
                mesh_unlit::transform_vertex(v, world);
            }
            self.mesh.write_prim_verts(queue, 0, prim, &verts);
        }

        let active = self_state.active_blaster();
        for b in &self.blasters {
            let root = if active == Some(b.letter) {
                // Parent to present-pose arm (hold + aim, or sprint swing).
                mesh_unlit::held_blaster_root(k2w, arm_kit, b.letter_index)
            } else {
                Mat4::from_scale(Vec3::ZERO)
            };
            for (pi, local) in b.local.iter().enumerate() {
                let mut verts = local.clone();
                for v in &mut verts {
                    mesh_unlit::transform_vertex(v, root);
                }
                self.mesh.write_prim_verts(queue, b.batch, pi, &verts);
            }
        }
    }

    /// Look pose: mount and aim (locomotion held at stand). Local self only.
    pub fn apply_look_view(&mut self, self_state: &SelfState) {
        let k2w = mesh_unlit::kit_to_world(self_state.placement_matrix(), self.min_y);
        let (look_worlds, _) = mesh_unlit::pose_character_kit(
            &self.parts,
            self_state,
            Some(&self.walk_clip),
            KitPose::Look,
        );
        let look_origin = look_origin_world(&self.parts, &look_worlds, k2w);
        let reticle_world = self_state.reticle_world(look_origin);

        self.view = MountedView {
            look_origin,
            reticle_world,
        };
    }

    pub fn apply_state(&mut self, queue: &wgpu::Queue, self_state: &SelfState) {
        self.apply_present(queue, self_state);
        self.apply_look_view(self_state);
    }

    pub fn write_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        self.mesh.write_view_proj(queue, view_proj);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.mesh.draw(pass);
    }
}

/// Face point on a kit pose in world space (look origin when pose is Look).
fn look_origin_world(parts: &[CharPart], worlds: &[Mat4], k2w: Mat4) -> Vec3 {
    let head_kit = parts
        .iter()
        .position(|p| p.name == "head")
        .map(|i| worlds[i])
        .unwrap_or(Mat4::IDENTITY);
    k2w.transform_point3(head_kit.transform_point3(FACE_OFFSET_HEAD_KIT))
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
