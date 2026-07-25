//! Per-frame simulation, combat, and draw for ClientInner.

mod combat;
mod draw;
mod play;
mod present;

use game_sim::{FireState, ProjectileWorld, SelfState};
use wasm_bindgen::prelude::*;

use crate::renderer::canvas_buffer_size;
use crate::self_present::SelfPresentState;

use super::ClientInner;

impl ClientInner {
    pub(crate) fn render_frame(&mut self) -> Result<(), JsValue> {
        let (width, height, ppp) = canvas_buffer_size(&self.canvas, self.renderer.max_texture_dim);
        self.pixels_per_point = ppp;
        if self.canvas.width() != width || self.canvas.height() != height {
            self.canvas.set_width(width);
            self.canvas.set_height(height);
        }
        self.renderer.resize_if_needed(width, height);

        let now = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now() / 1000.0)
            .unwrap_or(0.0);
        let dt = if self.last_frame_secs > 0.0 {
            (now - self.last_frame_secs).clamp(0.0, 0.1) as f32
        } else {
            1.0 / 60.0
        };
        self.last_frame_secs = now;

        #[cfg(feature = "debug-tools")]
        {
            let inst = if dt > 1e-6 { 1.0 / dt } else { 0.0 };
            if self.fps_ema <= 0.0 {
                self.fps_ema = inst;
            } else {
                self.fps_ema += 0.12 * (inst - self.fps_ema);
            }
            self.drain_debug_host_requests();
        }

        let look = self.session.take_look_px();
        let session_ok = self.session.is_active();
        #[cfg(feature = "debug-tools")]
        {
            self.last_look_px = look;
        }

        let effects = self.mp.drain_frame_effects();
        if effects.release_pointer_lock && self.session.is_active() {
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                doc.exit_pointer_lock();
            }
        }
        if let Some(spawn) = effects.pending_spawn {
            let character = self.mp.character();
            // SelfGpu only uploads blaster batches at load. Spawn applies staged
            // loadout; always re-kick so local present matches (remotes already
            // reload via RemoteKitKey). Idle also aborts a stale in-flight load.
            let need_present_reload = match &self.self_present {
                SelfPresentState::Ready(gpu) => {
                    self.self_state.character != character
                        || !gpu.covers_loadout(spawn.primary, spawn.secondary)
                }
                SelfPresentState::Loading | SelfPresentState::Failed(_) => true,
                SelfPresentState::Idle => false,
            };
            self.self_state = SelfState::default_loadout();
            self.self_state.character = character;
            self.self_state.primary = spawn.primary;
            self.self_state.secondary = spawn.secondary;
            self.self_state.active = spawn.active;
            self.self_state.position = spawn.position;
            self.self_state.set_look(spawn.yaw, 0.0);
            self.self_state.alive = true;
            self.self_state.health = game_sim::HEALTH_MAX;
            self.self_state.regen_block_s = 0.0;
            self.self_state.die_age_s = 0.0;
            self.fire = FireState::new();
            self.projectiles = ProjectileWorld::new();
            self.health_by_id.clear();
            self.ui.set_status(String::new());
            if need_present_reload {
                self.self_present = SelfPresentState::Idle;
            }
        }
        if let Some(err) = effects.error {
            #[cfg(feature = "debug-tools")]
            self.debug.shell.push_log(err.clone());
            self.ui.set_status(err);
        }

        #[cfg(feature = "debug-tools")]
        let console_open = self.debug.is_open();
        #[cfg(not(feature = "debug-tools"))]
        let console_open = false;

        #[cfg(feature = "debug-tools")]
        let debug_fly = self.debug.flycam_wanted();
        #[cfg(not(feature = "debug-tools"))]
        let debug_fly = false;

        let cam = self.mp.cam_intent(debug_fly);
        let was_fly = self.view.is_flycam();
        let play_ok = session_ok && !console_open && !cam.is_fly() && !self.mp.blocks_play();
        #[cfg(feature = "debug-tools")]
        {
            self.last_play_ok = play_ok;
        }
        let fire_held = self.tick_play_controls(dt, look, play_ok);

        self.tick_combat(dt, fire_held);

        self.tick_present(dt, session_ok, console_open, was_fly, look, cam);

        self.draw_frame(width, height, cam)
    }
}
