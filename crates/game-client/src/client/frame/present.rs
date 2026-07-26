//! Self/remote mesh pose and camera intent for one frame.

use crate::mp::CamIntent;
use crate::self_present::SelfPresentState;

use super::super::ClientInner;

impl ClientInner {
    pub(super) fn tick_present(
        &mut self,
        dt: f32,
        session_ok: bool,
        console_open: bool,
        was_fly: bool,
        look: glam::Vec2,
        cam: CamIntent,
    ) {
        // Body cascade → look on posed head → view mounts look (only FP path).
        let draw_local_self = self.mp.is_living();
        if draw_local_self {
            if let SelfPresentState::Ready(gpu) = &mut self.self_present {
                let first_person = !cam.is_fly();
                gpu.apply_state(&self.renderer.queue, &self.self_state, first_person);
                self.view
                    .set_mounted_look(gpu.view.look_origin, gpu.view.look_forward);
            }
        }

        if self.mp.in_room() {
            // Living remotes only — dead peers have no owned body except the corpse (059).
            let samples: Vec<_> = self
                .mp
                .remotes()
                .samples()
                .filter(|(id, _)| self.mp.peer_living(*id))
                .map(|(id, s)| (id, s.drive.clone()))
                .collect();
            self.remote_present.apply_all(
                &self.renderer.queue,
                samples.into_iter(),
                |id, state| {
                    self.renderer.fire_fx.apply_remote_fire_residual(id, state);
                },
                |_id| (true, 0.0),
            );
            self.corpse_present
                .apply_all(&self.renderer.queue, &self.world_loot.corpses);
        } else {
            self.remote_present.clear();
            self.corpse_present.clear();
        }

        let status = match cam {
            CamIntent::ProductFly => {
                if !self.view.is_flycam() {
                    self.view.enter_spectate_flycam();
                    Some("spectate flycam")
                } else {
                    None
                }
            }
            CamIntent::DebugFly => {
                let eye = self.view.mounted_eye();
                self.view.sync_fly_intent(true, &self.self_state, eye)
            }
            CamIntent::Mounted | CamIntent::Overview => {
                let eye = self.view.mounted_eye();
                self.view.sync_fly_intent(false, &self.self_state, eye)
            }
        };
        if let Some(msg) = status {
            #[cfg(feature = "debug-tools")]
            self.debug.shell.push_log(msg.to_string());
            #[cfg(not(feature = "debug-tools"))]
            let _ = msg;
            self.fly_input.clear_keys();
            self.move_input.clear_keys();
        }

        let flycam = self.view.is_flycam() && cam.is_fly();
        if session_ok && flycam && !console_open {
            let look = if was_fly { look } else { glam::Vec2::ZERO };
            self.view.update_flycam(dt, &self.fly_input, look);
        } else if console_open || !session_ok || !cam.is_fly() {
            self.fly_input.clear_keys();
        }
    }
}
