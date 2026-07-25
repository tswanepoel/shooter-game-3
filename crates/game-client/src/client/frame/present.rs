//! Self/remote mesh pose and debug flycam for one frame.

use crate::self_present::SelfPresentState;

use super::super::ClientInner;

impl ClientInner {
    #[cfg_attr(not(feature = "debug-tools"), allow(unused_variables))]
    pub(super) fn tick_present(
        &mut self,
        dt: f32,
        session_ok: bool,
        console_open: bool,
        was_fly: bool,
        look: glam::Vec2,
        want_fly: bool,
    ) {
        let draw_local_self = self.mp.is_solo() || self.mp.is_living();
        if draw_local_self {
            if let SelfPresentState::Ready(gpu) = &mut self.self_present {
                // Mounted FP hides local head (eye sits inside the shell). Flycam shows full body.
                #[cfg(feature = "debug-tools")]
                let first_person = !self.view.is_flycam();
                #[cfg(not(feature = "debug-tools"))]
                let first_person = true;
                gpu.apply_state(&self.renderer.queue, &self.self_state, first_person);
                self.view
                    .set_mounted_look(gpu.view.look_origin, gpu.view.look_forward);
            }
        }

        if self.mp.in_room() {
            let samples: Vec<_> = self
                .mp
                .remotes()
                .samples()
                .map(|(id, s)| (id, s.drive.clone()))
                .collect();
            self.remote_present.apply_all(
                &self.renderer.queue,
                samples.into_iter(),
                |id, state| {
                    self.renderer.fire_fx.apply_remote_fire_residual(id, state);
                },
                |id| {
                    self.health_by_id
                        .get(&id)
                        .map(|h| (h.alive, h.die_age_s))
                        .unwrap_or((true, 0.0))
                },
            );
        } else {
            self.remote_present.clear();
        }

        #[cfg(feature = "debug-tools")]
        {
            let mounted_eye = self.view.mounted_eye();
            if let Some(msg) = self
                .view
                .sync_fly_intent(want_fly, &self.self_state, mounted_eye)
            {
                self.debug.shell.push_log(msg.to_string());
                // Enter or leave: drop sticky WASD (held keys may only start
                // counting once flycam_wanted flips true mid-hold).
                self.fly_input.clear_keys();
                self.move_input.clear_keys();
            }

            let flycam = self.view.is_flycam();
            if session_ok && flycam && !console_open {
                // Enter frame already baked this look into self → fly pose; don't double-apply.
                let look = if was_fly { look } else { glam::Vec2::ZERO };
                self.view.update_flycam(dt, &self.fly_input, look);
            } else if console_open || !session_ok {
                self.fly_input.clear_keys();
            }
        }
    }
}
