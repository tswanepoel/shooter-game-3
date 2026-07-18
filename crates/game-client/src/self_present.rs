//! Self body + blaster presentation (013/014).

use game_sim::{SelfState, DEFAULT_BORE_RANGE_M, FACE_OFFSET_HEAD_KIT, RETICLE_CAM_NUDGE_M};
use glam::{Mat4, Vec3};
use wasm_bindgen::JsValue;

use crate::mesh_unlit::{self, CharPart, MeshVertex, UnlitMeshGpu, UnlitMeshLayout};

pub struct MountedView {
    pub eye: Vec3,
    pub reticle_world: Option<Vec3>,
}

pub struct SelfGpu {
    mesh: UnlitMeshGpu,
    parts: Vec<CharPart>,
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
        let blaster_prims =
            mesh_unlit::extract_primitives(blaster_glb).map_err(|e| JsValue::from_str(&e))?;

        let (worlds, arm_kit) = mesh_unlit::pose_character_kit(&parts, self_state);
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
        s.apply_state(queue, self_state, None);
        Ok(s)
    }

    pub fn apply_state(
        &mut self,
        queue: &wgpu::Queue,
        self_state: &SelfState,
        camera_eye: Option<Vec3>,
    ) {
        let (worlds, arm_kit) = mesh_unlit::pose_character_kit(&self.parts, self_state);
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

        let muzzle_kit =
            arm_kit.transform_point3(mesh_unlit::primary_muzzle_offset(self.letter_index));
        let muzzle = k2w.transform_point3(muzzle_kit);
        let weapon_dir = weapon_forward(self_state);

        let reticle_world = if self_state.alive && self_state.armed {
            let cam = camera_eye.unwrap_or(eye);
            compute_reticle(muzzle, weapon_dir, cam, self_state.position)
        } else {
            None
        };

        self.view = MountedView { eye, reticle_world };
    }

    pub fn write_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        self.mesh.write_view_proj(queue, view_proj);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.mesh.draw(pass);
    }
}

fn weapon_forward(s: &SelfState) -> Vec3 {
    let pitch = s.torso_pitch + s.shoulder_pitch;
    let cp = pitch.cos();
    Vec3::new(s.torso_yaw.sin() * cp, pitch.sin(), s.torso_yaw.cos() * cp)
}

fn compute_reticle(muzzle: Vec3, weapon_dir: Vec3, camera_eye: Vec3, feet: Vec3) -> Option<Vec3> {
    let dir = weapon_dir.normalize_or_zero();
    if dir.length_squared() < 1e-8 {
        return None;
    }

    let max_t = DEFAULT_BORE_RANGE_M;
    let near_skip = 0.12_f32;

    // Ground plane y = 0 (world). Skip near-muzzle and hits on local body volume.
    let mut hit_t = max_t;
    let mut on_ground = false;
    if dir.y.abs() > 1e-6 {
        let t = -muzzle.y / dir.y;
        if t > near_skip && t < max_t {
            let p = muzzle + dir * t;
            if !is_local_body_hit(p, feet) {
                hit_t = t;
                on_ground = true;
            }
        }
    }

    let aim = muzzle + dir * hit_t;
    let to_cam = (camera_eye - aim).normalize_or_zero();
    let nudged = if to_cam.length_squared() > 1e-8 {
        aim + to_cam * RETICLE_CAM_NUDGE_M
    } else {
        aim
    };

    // Camera-through-pixel occlusion: ground closer than aim (not the aim surface itself).
    if occluded_by_ground(camera_eye, nudged, on_ground.then_some(hit_t)) {
        return None;
    }

    Some(nudged)
}

fn is_local_body_hit(p: Vec3, feet: Vec3) -> bool {
    let d = p - feet;
    d.x.abs() < 0.6 && d.z.abs() < 0.6 && p.y > 0.0 && p.y < 1.9
}

fn occluded_by_ground(camera: Vec3, reticle: Vec3, aim_is_ground: Option<f32>) -> bool {
    let delta = reticle - camera;
    let dist = delta.length();
    if dist < 1e-4 {
        return false;
    }
    let dir = delta / dist;
    if dir.y.abs() < 1e-6 {
        return false;
    }
    let t = -camera.y / dir.y;
    if t <= 0.05 || t >= dist - 0.05 {
        return false;
    }
    // Aim surface (ground reticle) does not count as a blocker.
    if aim_is_ground.is_some() {
        return false;
    }
    true
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
