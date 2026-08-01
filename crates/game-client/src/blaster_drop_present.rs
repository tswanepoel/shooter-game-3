//! Floor blaster drop presents — Kenney letter mesh pinned at drop pose (067).

use std::collections::HashMap;

use glam::{Mat4, Quat, Vec3};
use std::f32::consts::FRAC_PI_2;
use wasm_bindgen::JsValue;

use crate::mesh::{self, LightPlate, UnlitMeshGpu, UnlitMeshLayout};

enum Slot {
    Loading { letter: u8 },
    Ready { gpu: UnlitMeshGpu, letter: u8 },
}

pub struct BlasterDropPresent {
    slots: HashMap<u64, Slot>,
}

impl BlasterDropPresent {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }

    pub fn plan_loads(
        &mut self,
        drops: &HashMap<u64, game_sim::BlasterDrop>,
    ) -> Vec<(u64, u8, Vec3)> {
        self.slots.retain(|id, _| drops.contains_key(id));
        let mut need = Vec::new();
        for (id, d) in drops {
            let reload = match self.slots.get(id) {
                Some(Slot::Loading { letter }) if *letter == d.letter => false,
                Some(Slot::Ready { letter, .. }) if *letter == d.letter => false,
                _ => true,
            };
            if reload {
                self.slots.insert(*id, Slot::Loading { letter: d.letter });
                need.push((*id, d.letter, d.position));
            }
        }
        need
    }

    pub fn finish_load(&mut self, drop_id: u64, letter: u8, result: Result<UnlitMeshGpu, JsValue>) {
        let still = matches!(
            self.slots.get(&drop_id),
            Some(Slot::Loading { letter: ch }) if *ch == letter
        );
        if !still {
            return;
        }
        match result {
            Ok(gpu) => {
                self.slots.insert(drop_id, Slot::Ready { gpu, letter });
            }
            Err(_) => {
                self.slots.remove(&drop_id);
            }
        }
    }

    pub fn write_view_proj_all(&self, queue: &wgpu::Queue, view_proj: Mat4, light: LightPlate) {
        for slot in self.slots.values() {
            if let Slot::Ready { gpu, .. } = slot {
                gpu.write_view_proj(queue, view_proj, light);
            }
        }
    }

    pub fn draw_all<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        for slot in self.slots.values() {
            if let Slot::Ready { gpu, .. } = slot {
                gpu.draw(pass);
            }
        }
    }
}

impl Default for BlasterDropPresent {
    fn default() -> Self {
        Self::new()
    }
}

/// Floor placement: roll about bore so the mesh lays on its side.
pub fn floor_root(position: Vec3) -> Mat4 {
    let rot = Quat::from_rotation_z(FRAC_PI_2);
    Mat4::from_rotation_translation(rot, position + Vec3::Y * 0.08)
}

pub async fn load_floor_blaster(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface_format: wgpu::TextureFormat,
    sample_count: u32,
    letter: u8,
    position: Vec3,
) -> Result<UnlitMeshGpu, JsValue> {
    let layout = UnlitMeshLayout::create(device, surface_format, sample_count);
    let pack = mesh::load_kenney_core().await?;
    let gpu = layout.upload_ctx(device, queue);
    let bl = letter as char;
    let blaster_glb = pack
        .get(&format!("blaster-{bl}.mesh"))
        .map_err(|e| JsValue::from_str(&e))?;
    let colormap = pack
        .get("blaster.colormap")
        .map_err(|e| JsValue::from_str(&e))?;
    let prims = mesh::extract_primitives(blaster_glb).map_err(|e| JsValue::from_str(&e))?;
    let batch = mesh::upload_batch(
        &gpu,
        colormap,
        prims,
        floor_root(position),
        &format!("floor-blaster-{bl}"),
    )
    .map_err(|e| JsValue::from_str(&e))?;
    Ok(layout.finish(vec![batch]))
}
