//! Corpse kit presents — die-hold bodies separate from living remotes (059).

use std::collections::HashMap;

use game_sim::{LocomotionMode, SelfState};
use glam::Vec3;
use wasm_bindgen::JsValue;

use crate::mesh::LightPlate;
use crate::self_present::SelfGpu;
use crate::world_loot::WorldCorpse;

enum Slot {
    Loading { character: u8 },
    Ready { gpu: SelfGpu, character: u8 },
}

pub struct CorpsePresent {
    slots: HashMap<u64, Slot>,
}

impl CorpsePresent {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }

    pub fn plan_loads(&mut self, corpses: &HashMap<u64, WorldCorpse>) -> Vec<(u64, u8)> {
        self.slots.retain(|id, _| corpses.contains_key(id));
        let mut need = Vec::new();
        for (id, c) in corpses {
            let reload = match self.slots.get(id) {
                Some(Slot::Loading { character }) if *character == c.character => false,
                Some(Slot::Ready { character, .. }) if *character == c.character => false,
                _ => true,
            };
            if reload {
                self.slots.insert(
                    *id,
                    Slot::Loading {
                        character: c.character,
                    },
                );
                need.push((*id, c.character));
            }
        }
        need
    }

    pub fn finish_load(&mut self, corpse_id: u64, character: u8, result: Result<SelfGpu, JsValue>) {
        let still = matches!(
            self.slots.get(&corpse_id),
            Some(Slot::Loading { character: ch }) if *ch == character
        );
        if !still {
            return;
        }
        match result {
            Ok(gpu) => {
                self.slots.insert(corpse_id, Slot::Ready { gpu, character });
            }
            Err(_) => {
                self.slots.remove(&corpse_id);
            }
        }
    }

    pub fn apply_all(&mut self, queue: &wgpu::Queue, corpses: &HashMap<u64, WorldCorpse>) {
        for (id, c) in corpses {
            let Some(Slot::Ready { gpu, character }) = self.slots.get_mut(id) else {
                continue;
            };
            if *character != c.character {
                continue;
            }
            let mut state = SelfState::default_loadout();
            state.position = c.position;
            state.facing = c.facing;
            state.character = c.character;
            state.alive = false;
            state.die_age_s = c.die_age_s;
            state.primary = None;
            state.secondary = None;
            state.clear_emote();
            state.locomotion = LocomotionMode::Stand;
            state.walk_phase = 0.0;
            gpu.apply_present(queue, &state, false);
        }
    }

    pub fn write_view_proj_all(
        &self,
        queue: &wgpu::Queue,
        view_proj: glam::Mat4,
        light: LightPlate,
    ) {
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

impl Default for CorpsePresent {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a minimal state for async kit load.
pub fn corpse_load_state(character: u8, position: Vec3, facing: f32) -> SelfState {
    let mut state = SelfState::default_loadout();
    state.character = character;
    state.position = position;
    state.facing = facing;
    state.alive = false;
    state.die_age_s = game_sim::DIE_DURATION_S;
    state.primary = None;
    state.secondary = None;
    state
}
