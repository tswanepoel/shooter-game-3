//! Kit meshes for remote peers; same present path as local self.

use std::collections::HashMap;

use game_net::{DriveView, PlayerId};
use game_sim::SelfState;
use glam::Vec3;
use wasm_bindgen::JsValue;

use crate::body_hit::PartHit;
use crate::mp::{drive_to_state, RemoteKitKey};
use crate::self_present::SelfGpu;

enum Slot {
    Loading { kit: RemoteKitKey },
    Ready { gpu: SelfGpu, kit: RemoteKitKey },
    Failed,
}

pub struct RemotePresent {
    slots: HashMap<PlayerId, Slot>,
}

impl RemotePresent {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }

    pub fn plan_loads_from(
        &mut self,
        live_ids: &[PlayerId],
        samples: &[(PlayerId, DriveView)],
    ) -> Vec<(PlayerId, SelfState, RemoteKitKey)> {
        self.slots.retain(|id, _| live_ids.contains(id));

        let mut need = Vec::new();
        for (id, drive) in samples {
            let kit = RemoteKitKey::from_drive(drive);
            let reload = match self.slots.get(id) {
                Some(Slot::Loading { kit: k }) if *k == kit => false,
                Some(Slot::Ready { kit: k, .. }) if *k == kit => false,
                Some(Slot::Failed) => false,
                _ => true,
            };
            if reload {
                self.slots.insert(*id, Slot::Loading { kit });
                need.push((*id, drive_to_state(drive), kit));
            }
        }
        need
    }

    pub fn finish_load(
        &mut self,
        id: PlayerId,
        kit: RemoteKitKey,
        result: Result<SelfGpu, JsValue>,
    ) {
        let still = matches!(
            self.slots.get(&id),
            Some(Slot::Loading { kit: k }) if *k == kit
        );
        if !still {
            return;
        }
        match result {
            Ok(gpu) => {
                self.slots.insert(id, Slot::Ready { gpu, kit });
            }
            Err(_) => {
                self.slots.insert(id, Slot::Failed);
            }
        }
    }

    pub fn apply_all(
        &mut self,
        queue: &wgpu::Queue,
        samples: impl Iterator<Item = (PlayerId, DriveView)>,
        mut apply_residual: impl FnMut(PlayerId, &mut SelfState),
        death_for: impl Fn(PlayerId) -> (bool, f32),
    ) {
        for (id, drive) in samples {
            let Some(Slot::Ready { gpu, kit }) = self.slots.get_mut(&id) else {
                continue;
            };
            if *kit != RemoteKitKey::from_drive(&drive) {
                continue;
            }
            let mut state = drive_to_state(&drive);
            let (alive, die_age_s) = death_for(id);
            state.alive = alive;
            state.die_age_s = die_age_s;
            if !alive {
                state.clear_emote();
                state.sprint_latched = false;
                if !state.locomotion.is_air() {
                    state.locomotion = game_sim::LocomotionMode::Stand;
                    state.walk_phase = 0.0;
                }
            }
            apply_residual(id, &mut state);
            gpu.apply_present(queue, &state, false);
        }
    }

    pub fn write_view_proj_all(&self, queue: &wgpu::Queue, view_proj: glam::Mat4) {
        for slot in self.slots.values() {
            if let Slot::Ready { gpu, .. } = slot {
                gpu.write_view_proj(queue, view_proj);
            }
        }
    }

    pub fn iter_name_anchors(&self) -> impl Iterator<Item = (PlayerId, Vec3)> + '_ {
        const ABOVE_HEAD_M: f32 = 0.80;
        self.slots.iter().filter_map(|(&id, slot)| {
            let Slot::Ready { gpu, .. } = slot else {
                return None;
            };
            Some((id, gpu.view.head_joint_world + Vec3::Y * ABOVE_HEAD_M))
        })
    }

    pub fn flash_muzzle_world(
        &self,
        id: PlayerId,
        state: &SelfState,
        grip_bore_m: f32,
        muzzle_index: u8,
    ) -> Option<glam::Vec3> {
        let Slot::Ready { gpu, .. } = self.slots.get(&id)? else {
            return None;
        };
        gpu.flash_muzzle_worlds_with_bore(state, grip_bore_m, &[muzzle_index])
            .into_iter()
            .next()
    }

    pub fn trace_segment(
        &self,
        id: PlayerId,
        state: &SelfState,
        from: Vec3,
        to: Vec3,
    ) -> Option<PartHit> {
        let Slot::Ready { gpu, .. } = self.slots.get(&id)? else {
            return None;
        };
        if !state.alive {
            return None;
        }
        gpu.trace_segment(state, from, to)
    }

    pub fn draw_all<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        for slot in self.slots.values() {
            if let Slot::Ready { gpu, .. } = slot {
                gpu.draw(pass);
            }
        }
    }
}

impl Default for RemotePresent {
    fn default() -> Self {
        Self::new()
    }
}
