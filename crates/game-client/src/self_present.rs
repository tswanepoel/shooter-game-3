//! Self body + blaster: one posed figure; local view on that head's eye socket.

use game_sim::{emote_clip_name, SelfState, EMOTE_CATALOG, FACE_OFFSET_HEAD_KIT};
use glam::{Mat4, Vec3};
use wasm_bindgen::JsValue;

use crate::body_hit::{self, PartHit};
use crate::mesh::{self, AnimClip, CharPart, MeshVertex, UnlitMeshGpu, UnlitMeshLayout};

pub struct MountedView {
    pub look_origin: Vec3,
    pub look_forward: Vec3,
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
    die_clip: AnimClip,
    /// Wheel emote clips, index = slot id (039).
    emote_clips: Vec<AnimClip>,
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
        let pack = mesh::load_kenney_core().await?;
        let gpu = layout.upload_ctx(device, queue);

        let ch = self_state.character as char;

        let char_glb = pack
            .get(&format!("character-{ch}.mesh"))
            .map_err(|e| JsValue::from_str(&e))?;
        let char_png = pack
            .get(&format!("character-{ch}.albedo"))
            .map_err(|e| JsValue::from_str(&e))?;

        let (parts, min_y) =
            mesh::extract_character_parts(char_glb).map_err(|e| JsValue::from_str(&e))?;
        let walk_clip = mesh::extract_clip(char_glb, "walk").map_err(|e| JsValue::from_str(&e))?;
        let sprint_clip =
            mesh::extract_clip(char_glb, "sprint").map_err(|e| JsValue::from_str(&e))?;
        let die_clip = mesh::extract_clip(char_glb, "die").map_err(|e| JsValue::from_str(&e))?;
        let mut emote_clips = Vec::with_capacity(EMOTE_CATALOG.len());
        for def in &EMOTE_CATALOG {
            let clip = mesh::extract_clip(char_glb, def.clip).map_err(|e| JsValue::from_str(&e))?;
            emote_clips.push(clip);
        }

        let loco = if self_state.locomotion.is_sprint() {
            &sprint_clip
        } else {
            &walk_clip
        };
        let emote = emote_pair(self_state, &emote_clips);
        let die = die_pair(self_state, &die_clip);
        let (worlds, arm_kit) =
            mesh::pose_character_kit(&parts, self_state, Some(loco), emote, die);
        let k2w = mesh::kit_to_world(self_state.placement_matrix(), min_y);

        let mut char_cpu: Vec<(Vec<MeshVertex>, Vec<u32>, [f32; 4])> = Vec::new();
        let mut part_prim = vec![None; parts.len()];
        for (i, part) in parts.iter().enumerate() {
            if part.local_verts.is_empty() {
                continue;
            }
            let world = k2w * worlds[i];
            let mut verts = part.local_verts.clone();
            for v in &mut verts {
                mesh::transform_vertex(v, world);
            }
            part_prim[i] = Some(char_cpu.len());
            char_cpu.push((verts, part.indices.clone(), part.base_color));
        }

        let char_batch =
            mesh::upload_batch(&gpu, char_png, char_cpu, Mat4::IDENTITY, "self-character")
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
            let bi = mesh::letter_index(letter).map_err(|e| JsValue::from_str(&e))?;
            let bl = letter as char;
            let blaster_glb = pack
                .get(&format!("blaster-{bl}.mesh"))
                .map_err(|e| JsValue::from_str(&e))?;
            let colormap = pack
                .get("blaster.colormap")
                .map_err(|e| JsValue::from_str(&e))?;
            let blaster_prims =
                mesh::extract_primitives(blaster_glb).map_err(|e| JsValue::from_str(&e))?;

            let show = self_state.presents_armed() && self_state.active_blaster() == Some(letter);
            let root = if show {
                mesh::held_blaster_root(k2w, arm_kit, bi)
            } else {
                Mat4::from_scale(Vec3::ZERO)
            };

            let mut local = Vec::new();
            let mut cpu = Vec::new();
            for (verts, indices, color) in blaster_prims {
                local.push(verts.clone());
                let mut world_verts = verts;
                for v in &mut world_verts {
                    mesh::transform_vertex(v, root);
                }
                cpu.push((world_verts, indices, color));
            }
            let batch_idx = batches.len();
            let batch = mesh::upload_batch(
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
            die_clip,
            emote_clips,
            part_prim,
            blasters,
            min_y,
            view: MountedView {
                look_origin: Vec3::ZERO,
                look_forward: Vec3::Z,
                reticle_world: None,
            },
        };
        // Full body at load (remotes share this path); local FP hides head next apply.
        s.apply_state(queue, self_state, false);
        Ok(s)
    }

    /// Present-pose arm matrix and kit→world (for held attach / fire origins).
    fn present_arm(&self, self_state: &SelfState) -> (Mat4, Mat4) {
        let k2w = mesh::kit_to_world(self_state.placement_matrix(), self.min_y);
        let loco = if self_state.locomotion.is_sprint() {
            &self.sprint_clip
        } else {
            &self.walk_clip
        };
        let emote = emote_pair(self_state, &self.emote_clips);
        let die = die_pair(self_state, &self.die_clip);
        let (_, arm_kit) =
            mesh::pose_character_kit(&self.parts, self_state, Some(loco), emote, die);
        (k2w, arm_kit)
    }

    pub fn fire_muzzle_worlds(&self, self_state: &SelfState) -> Vec<Vec3> {
        let Some(letter) = self_state.active_blaster() else {
            return Vec::new();
        };
        let Ok(bi) = mesh::letter_index(letter) else {
            return Vec::new();
        };
        let (k2w, arm_kit) = self.present_arm(self_state);
        let held = held_with_grip_bore(k2w, arm_kit, bi, self_state.grip_bore_m);
        mesh::muzzle_world_points(held, bi).collect()
    }

    pub fn flash_muzzle_worlds(&self, self_state: &SelfState, muzzle_indices: &[u8]) -> Vec<Vec3> {
        let Some(letter) = self_state.active_blaster() else {
            return Vec::new();
        };
        let Ok(bi) = mesh::letter_index(letter) else {
            return Vec::new();
        };
        let (k2w, arm_kit) = self.present_arm(self_state);
        let held = held_with_grip_bore(k2w, arm_kit, bi, self_state.grip_bore_m);
        let locals = mesh::muzzle_locals(bi);
        muzzle_indices
            .iter()
            .filter_map(|&i| {
                let i = i as usize;
                locals
                    .get(i)
                    .map(|p| held.transform_point3(Vec3::from_array(*p)))
            })
            .collect()
    }

    pub fn flash_muzzle_worlds_with_bore(
        &self,
        self_state: &SelfState,
        grip_bore_m: f32,
        muzzle_indices: &[u8],
    ) -> Vec<Vec3> {
        let Some(letter) = self_state.active_blaster() else {
            return Vec::new();
        };
        let Ok(bi) = mesh::letter_index(letter) else {
            return Vec::new();
        };
        let (k2w, arm_kit) = self.present_arm(self_state);
        let held = held_with_grip_bore(k2w, arm_kit, bi, grip_bore_m);
        let locals = mesh::muzzle_locals(bi);
        muzzle_indices
            .iter()
            .filter_map(|&i| {
                let i = i as usize;
                locals
                    .get(i)
                    .map(|p| held.transform_point3(Vec3::from_array(*p)))
            })
            .collect()
    }

    pub fn trace_segment(&self, self_state: &SelfState, from: Vec3, to: Vec3) -> Option<PartHit> {
        let loco = if self_state.locomotion.is_sprint() {
            &self.sprint_clip
        } else {
            &self.walk_clip
        };
        let emote = emote_pair(self_state, &self.emote_clips);
        let die = die_pair(self_state, &self.die_clip);
        body_hit::trace_segment_parts(
            &self.parts,
            self_state,
            Some(loco),
            emote,
            die,
            self.min_y,
            from,
            to,
        )
    }

    /// Body + active blaster from drive; mounts view on the posed head.
    /// `first_person` hides the head shell.
    pub fn apply_present(
        &mut self,
        queue: &wgpu::Queue,
        self_state: &SelfState,
        first_person: bool,
    ) {
        self.apply_present_with_bore(queue, self_state, self_state.grip_bore_m, first_person);
    }

    pub fn apply_present_with_bore(
        &mut self,
        queue: &wgpu::Queue,
        self_state: &SelfState,
        grip_bore_m: f32,
        first_person: bool,
    ) {
        let k2w = mesh::kit_to_world(self_state.placement_matrix(), self.min_y);

        let loco = if self_state.locomotion.is_sprint() {
            &self.sprint_clip
        } else {
            &self.walk_clip
        };
        let emote = emote_pair(self_state, &self.emote_clips);
        let die = die_pair(self_state, &self.die_clip);
        let (present_worlds, arm_kit) =
            mesh::pose_character_kit(&self.parts, self_state, Some(loco), emote, die);

        for (i, part) in self.parts.iter().enumerate() {
            let Some(prim) = self.part_prim[i] else {
                continue;
            };
            // Near plane only drops geometry closer than CAMERA_NEAR_M; the head
            // shell sits past that, so FP must not draw it at all.
            let world = if first_person && part.name == "head" {
                Mat4::from_scale(Vec3::ZERO)
            } else {
                k2w * present_worlds[i]
            };
            let mut verts = part.local_verts.clone();
            for v in &mut verts {
                mesh::transform_vertex(v, world);
            }
            self.mesh.write_prim_verts(queue, 0, prim, &verts);
        }

        let show_gun = self_state.presents_armed();
        let active = self_state.active_blaster();
        for b in &self.blasters {
            let root = if show_gun && active == Some(b.letter) {
                held_with_grip_bore(k2w, arm_kit, b.letter_index, grip_bore_m)
            } else {
                Mat4::from_scale(Vec3::ZERO)
            };
            for (pi, local) in b.local.iter().enumerate() {
                let mut verts = local.clone();
                for v in &mut verts {
                    mesh::transform_vertex(v, root);
                }
                self.mesh.write_prim_verts(queue, b.batch, pi, &verts);
            }
        }

        let (look_origin, look_forward) = look_from_head(&self.parts, &present_worlds, k2w);
        let reticle_world = self_state.reticle_world(look_origin);
        self.view = MountedView {
            look_origin,
            look_forward,
            reticle_world,
        };
    }

    pub fn apply_state(&mut self, queue: &wgpu::Queue, self_state: &SelfState, first_person: bool) {
        self.apply_present(queue, self_state, first_person);
    }

    /// Whether both loadout letters are already GPU-resident (dev equip).
    #[cfg(feature = "debug-tools")]
    pub fn has_blaster_letter(&self, letter: u8) -> bool {
        self.blasters.iter().any(|b| b.letter == letter)
    }

    pub fn write_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        self.mesh.write_view_proj(queue, view_proj);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.mesh.draw(pass);
    }
}

fn held_with_grip_bore(k2w: Mat4, arm_kit: Mat4, letter_index: usize, grip_bore_m: f32) -> Mat4 {
    let held = mesh::held_blaster_root(k2w, arm_kit, letter_index);
    if grip_bore_m.abs() < 1e-8 {
        return held;
    }
    let grip = mesh::weapon_grip(letter_index).transform_point3(Vec3::ZERO);
    let t = Mat4::from_translation(grip);
    held * t * Mat4::from_translation(Vec3::new(0.0, 0.0, grip_bore_m)) * t.inverse()
}

/// Eye socket on posed head; forward is kit face (+Z).
fn look_from_head(parts: &[CharPart], worlds: &[Mat4], k2w: Mat4) -> (Vec3, Vec3) {
    let head_kit = parts
        .iter()
        .position(|p| p.name == "head")
        .map(|i| worlds[i])
        .unwrap_or(Mat4::IDENTITY);
    let head_world = k2w * head_kit;
    let origin = head_world.transform_point3(FACE_OFFSET_HEAD_KIT);
    let forward = head_world.transform_vector3(Vec3::Z).normalize_or_zero();
    let forward = if forward.length_squared() < 1e-12 {
        Vec3::Z
    } else {
        forward
    };
    (origin, forward)
}

fn emote_pair<'a>(self_state: &SelfState, clips: &'a [AnimClip]) -> Option<(&'a AnimClip, f32)> {
    if !self_state.alive {
        return None;
    }
    let id = self_state.emote?;
    let _ = emote_clip_name(id)?;
    let clip = clips.get(id as usize)?;
    Some((clip, self_state.emote_age_s))
}

fn die_pair<'a>(self_state: &SelfState, die_clip: &'a AnimClip) -> Option<(&'a AnimClip, f32)> {
    if self_state.alive {
        return None;
    }
    Some((die_clip, self_state.die_age_s))
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
