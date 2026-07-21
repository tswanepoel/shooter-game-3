//! Kit meshes for remote peers; same present path as local self.

use std::collections::HashMap;

use game_net::{DriveView, PlayerId};
use game_sim::SelfState;
use wasm_bindgen::JsValue;

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
    ) {
        for (id, drive) in samples {
            let Some(Slot::Ready { gpu, kit }) = self.slots.get_mut(&id) else {
                continue;
            };
            if *kit != RemoteKitKey::from_drive(&drive) {
                continue;
            }
            let state = drive_to_state(&drive);
            gpu.apply_present(queue, &state);
        }
    }

    pub fn write_view_proj_all(&self, queue: &wgpu::Queue, view_proj: glam::Mat4) {
        for slot in self.slots.values() {
            if let Slot::Ready { gpu, .. } = slot {
                gpu.write_view_proj(queue, view_proj);
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

impl Default for RemotePresent {
    fn default() -> Self {
        Self::new()
    }
}
