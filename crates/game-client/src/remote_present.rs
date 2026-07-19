//! Third-person present bodies for remote peers (024).
//!
//! `mp/remotes` owns the pose table; this module loads kit meshes and feeds the
//! same present path as local self (`SelfGpu::apply_present`).

use std::collections::HashMap;

use game_net::PlayerId;
use game_sim::SelfState;
use wasm_bindgen::JsValue;

use crate::mp::{pose_to_state, RemoteKitKey, RemoteTable};
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

    /// Drop peers not in `live_ids`. Returns loads to kick for new/changed kits.
    pub fn plan_loads(
        &mut self,
        live_ids: &[PlayerId],
        poses: &[game_net::NetPlayerPose],
    ) -> Vec<(PlayerId, SelfState, RemoteKitKey)> {
        self.slots.retain(|id, _| live_ids.contains(id));

        let mut need = Vec::new();
        for pose in poses {
            let kit = RemoteKitKey::from_pose(pose);
            let reload = match self.slots.get(&pose.id) {
                Some(Slot::Loading { kit: k }) if *k == kit => false,
                Some(Slot::Ready { kit: k, .. }) if *k == kit => false,
                Some(Slot::Failed) => false,
                _ => true,
            };
            if reload {
                self.slots.insert(pose.id, Slot::Loading { kit });
                need.push((pose.id, pose_to_state(pose), kit));
            }
        }
        need
    }

    /// Finish an async load if the peer is still expected with the same kit.
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

    /// Present-pose each ready remote from clock-sampled poses (027 / 028).
    pub fn apply_all(&mut self, queue: &wgpu::Queue, table: &RemoteTable) {
        for pose in table.present_poses() {
            let Some(Slot::Ready { gpu, kit }) = self.slots.get_mut(&pose.id) else {
                continue;
            };
            if *kit != RemoteKitKey::from_pose(&pose) {
                continue;
            }
            let state = pose_to_state(&pose);
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
