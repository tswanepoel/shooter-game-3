//! Camera, scene pass, overlays, and UI for one frame.

use wasm_bindgen::prelude::*;

#[cfg(feature = "debug-tools")]
use crate::lineup::LineupState;
use crate::mp::{self, CamIntent};
use crate::self_present::SelfPresentState;
use crate::ui_overlay::{DebugDraw, FloatingNameLabel, OverlayGpu, ProductSession};
use crate::view::overview_view_matrix;

#[cfg(feature = "debug-tools")]
use super::super::load::capture_canvas_png;
use super::super::ClientInner;

impl ClientInner {
    pub(super) fn draw_frame(
        &mut self,
        width: u32,
        height: u32,
        cam: CamIntent,
    ) -> Result<(), JsValue> {
        let flycam = cam.is_fly();
        let draw_local_self = self.mp.is_living();

        let (cam_eye, cam_fwd) = self.view.eye_and_forward();
        let view_mat = if cam == CamIntent::Overview {
            overview_view_matrix()
        } else {
            self.view.view_matrix()
        };
        let view_proj = self.renderer.write_view_proj(view_mat);

        let reticle_pos = match &self.self_present {
            SelfPresentState::Ready(gpu) => gpu.view.reticle_world,
            _ => None,
        };
        self.renderer.reticle.update(
            &self.renderer.queue,
            view_proj,
            reticle_pos,
            cam_eye,
            cam_fwd,
            height as f32,
        );

        {
            let remote_states: std::collections::HashMap<_, _> = self
                .mp
                .remotes()
                .samples()
                .map(|(id, s)| {
                    let mut state = mp::drive_to_state(&s.drive);
                    self.renderer
                        .fire_fx
                        .apply_remote_fire_residual(id, &mut state);
                    (id, state)
                })
                .collect();
            let self_state = &self.self_state;
            let self_present = &self.self_present;
            let remote_present = &self.remote_present;
            self.renderer
                .fire_fx
                .rebind_positions(|owner, mi| match owner {
                    None => match self_present {
                        SelfPresentState::Ready(gpu) => gpu
                            .flash_muzzle_worlds(self_state, &[mi])
                            .into_iter()
                            .next(),
                        _ => None,
                    },
                    Some(id) => {
                        let state = remote_states.get(&id)?;
                        remote_present.flash_muzzle_world(id, state, state.grip_bore_m, mi)
                    }
                });
        }

        self.renderer.fire_fx.update_draw(
            &self.renderer.queue,
            view_proj,
            cam_eye,
            cam_fwd,
            &self.projectiles.projectiles,
        );

        if draw_local_self {
            if let SelfPresentState::Ready(gpu) = &self.self_present {
                gpu.write_view_proj(&self.renderer.queue, view_proj);
            }
        }
        if self.mp.in_room() {
            self.remote_present
                .write_view_proj_all(&self.renderer.queue, view_proj);
            self.corpse_present
                .write_view_proj_all(&self.renderer.queue, view_proj);
        }
        let self_ref = if draw_local_self {
            match &self.self_present {
                SelfPresentState::Ready(gpu) => Some(gpu),
                _ => None,
            }
        } else {
            None
        };
        let remotes_ref = if self.mp.in_room() {
            Some(&self.remote_present)
        } else {
            None
        };
        let corpses_ref = if self.mp.in_room() {
            Some(&self.corpse_present)
        } else {
            None
        };

        #[cfg(feature = "debug-tools")]
        let draw_grid = self.debug.draw_grid();
        #[cfg(not(feature = "debug-tools"))]
        let draw_grid = true;

        let draw_reticle = reticle_pos.is_some() && !flycam;
        let hit_alpha = self.hit_marker.alpha();
        let draw_hit_marker = hit_alpha > 0.0 && reticle_pos.is_some() && !flycam;
        self.renderer.hit_marker_gpu.update(
            &self.renderer.queue,
            view_proj,
            if draw_hit_marker { reticle_pos } else { None },
            cam_eye,
            cam_fwd,
            height as f32,
            if draw_hit_marker { hit_alpha } else { 0.0 },
        );
        let draw_emote_wheel = self.emote_wheel.is_open() && !flycam;
        let aspect = self.renderer.config.width as f32 / self.renderer.config.height.max(1) as f32;
        self.renderer.emote_wheel_gpu.update(
            &self.renderer.queue,
            draw_emote_wheel,
            self.emote_wheel.highlighted_slot(),
            aspect,
        );

        #[cfg(feature = "debug-tools")]
        let frame = {
            let want_lineup = self.debug.draw_lineup();
            if want_lineup {
                if let LineupState::Ready(gpu) = &self.lineup {
                    gpu.write_view_proj(&self.renderer.queue, view_proj);
                }
            }
            let lineup_ref = match &self.lineup {
                LineupState::Ready(gpu) if want_lineup => Some(gpu),
                _ => None,
            };
            self.renderer.render_scene(
                draw_grid,
                self_ref,
                remotes_ref,
                corpses_ref,
                lineup_ref,
                draw_reticle,
                draw_hit_marker,
                draw_emote_wheel,
            )?
        };
        #[cfg(not(feature = "debug-tools"))]
        let frame = self.renderer.render_scene(
            draw_grid,
            self_ref,
            remotes_ref,
            corpses_ref,
            draw_reticle,
            draw_hit_marker,
            draw_emote_wheel,
        )?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        {
            let ppp = self.pixels_per_point;
            let time = web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now() / 1000.0)
                .unwrap_or(0.0);
            let screen_w = width as f32 / ppp;
            let screen_h = height as f32 / ppp;

            let roster = self.mp.roster();
            let phase = self.mp.phase();
            let connecting = self.mp.is_connecting();
            let floating_names = collect_floating_names(
                &self.remote_present,
                &roster,
                self.mp.player_id(),
                |id| self.mp.peer_living(id),
                cam_eye,
                view_proj,
                screen_w,
                screen_h,
            );
            let raw = self.ui.take_raw_input(screen_w, screen_h, time);

            #[cfg(feature = "debug-tools")]
            self.debug.apply_toggle();

            #[cfg(feature = "debug-tools")]
            let hud_owned = {
                let net = self.debug.net_hud();
                let residual = self.debug.residual_hud();
                if !net && !residual {
                    None
                } else {
                    let mut parts: Vec<String> = Vec::new();
                    if net {
                        let mut line = format!("fps {:.0}", self.fps_ema);
                        if let Some(tick) = self.mp.hud_tick_field() {
                            line.push_str("  ");
                            line.push_str(&tick);
                        }
                        parts.push(line);
                    }
                    if residual {
                        let cont = self.fire.fire_continues();
                        let fall_ms = self.self_state.fire_fall_eff_s(cont) * 1000.0;
                        parts.push(format!(
                            "fF {:.2}°  fT {:.2}°  fall {:.0}ms{}",
                            self.self_state.fire_fold_total().to_degrees(),
                            self.self_state.shoulder_fire_twist.to_degrees(),
                            fall_ms,
                            if cont { "  cont" } else { "" },
                        ));
                    }
                    if let (Some(mag), Some(cap)) = (
                        self.self_state.active_mag(),
                        self.self_state.active_mag_capacity(),
                    ) {
                        let rsv = self
                            .self_state
                            .active_ammo_kind()
                            .map(|k| self.self_state.reserve.get(k))
                            .unwrap_or(0);
                        parts.push(format!("mag {mag}/{cap}  rsv {rsv}"));
                    }
                    // 059: see whether a drop exists and how far (take radius 1.5m).
                    if let Some((n, dist, rounds)) =
                        self.world_loot.hud_near(self.self_state.position)
                    {
                        parts.push(format!("loot {n} near {dist:.2}m r={rounds}"));
                    } else {
                        parts.push(format!("loot 0  corpses {}", self.world_loot.corpses.len()));
                    }
                    Some(parts.join("  |  "))
                }
            };

            #[cfg(feature = "debug-tools")]
            let debug_draw = {
                let hud = hud_owned.as_deref();
                if self.debug.shell.wants_draw(hud) {
                    DebugDraw::Shell {
                        shell: &mut self.debug.shell,
                        hud,
                    }
                } else {
                    DebugDraw::none()
                }
            };
            #[cfg(not(feature = "debug-tools"))]
            let debug_draw = DebugDraw::none();

            let (full, actions) = self.ui.run(
                raw,
                ppp,
                ProductSession {
                    phase,
                    roster: &roster,
                    connecting,
                    character: self.mp.character(),
                    staged: self.mp.staged_loadout(),
                    floating_names: &floating_names,
                },
                debug_draw,
            );

            if let Some((room, name)) = actions.join {
                self.ui.set_status("joining…");
                self.mp.begin_join_with(&room, &name);
            }
            if actions.play {
                self.ui.sync_pick_character(self.mp.character());
                self.mp.choose_play();
            }
            if actions.spectate {
                self.mp.choose_spectate();
            }
            if actions.back_to_role {
                self.mp.back_to_role();
            }
            if let Some(ch) = actions.confirm_character {
                if let Some(committed) = self.mp.confirm_character(ch) {
                    let kit_changed = self.self_state.character != committed;
                    self.self_state.character = committed;
                    if kit_changed {
                        self.self_present = SelfPresentState::Idle;
                    }
                    self.ui.sync_pick_character(committed);
                }
            }
            if let Some(p) = actions.stage_primary {
                let _ = self.mp.stage_primary(p);
            }
            if let Some(s) = actions.stage_secondary {
                let _ = self.mp.stage_secondary(s);
            }
            if let Some(a) = actions.stage_active {
                self.mp.stage_active(a);
            }
            if actions.spawn {
                self.mp.request_spawn();
            }
            if actions.leave {
                self.mp.leave();
                self.remote_present.clear();
                self.corpse_present.clear();
                self.world_loot.clear();
                self.health_by_id.clear();
                self.ui.set_status(String::new());
            }

            #[cfg(feature = "debug-tools")]
            {
                if let Some(cmd) = self.debug.shell.take_pending_command() {
                    let _ = self.debug.execute(&cmd);
                }
                self.drain_debug_host_requests();
            }

            if let Some(full) = full {
                let mut encoder =
                    self.renderer
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("ui-encoder"),
                        });
                self.ui.render(
                    OverlayGpu {
                        device: &self.renderer.device,
                        queue: &self.renderer.queue,
                        encoder: &mut encoder,
                        view: &view,
                        width,
                        height,
                    },
                    full,
                );
                self.renderer
                    .queue
                    .submit(std::iter::once(encoder.finish()));
            }
        }

        frame.present();

        #[cfg(feature = "debug-tools")]
        {
            if self.debug.take_screenshot_request() {
                if let Err(err) = capture_canvas_png(&self.canvas) {
                    web_sys::console::error_1(&err);
                    self.debug.shell.push_log(format!(
                        "screenshot failed: {}",
                        err.as_string().unwrap_or_default()
                    ));
                } else {
                    self.debug.shell.push_log("screenshot ok");
                }
            }
        }

        Ok(())
    }
}

fn collect_floating_names(
    remotes: &crate::remote_present::RemotePresent,
    roster: &[game_net::RosterEntry],
    local_id: Option<game_net::PlayerId>,
    peer_living: impl Fn(game_net::PlayerId) -> bool,
    cam_eye: glam::Vec3,
    view_proj: glam::Mat4,
    screen_w: f32,
    screen_h: f32,
) -> Vec<FloatingNameLabel> {
    const REF_DIST_M: f32 = 8.0;
    const REF_SIZE: f32 = 16.0;
    const MIN_SIZE: f32 = 10.0;
    const MAX_SIZE: f32 = 22.0;

    let mut out = Vec::new();
    for (id, world) in remotes.iter_name_anchors() {
        if Some(id) == local_id || !peer_living(id) {
            continue;
        }
        let Some(entry) = roster.iter().find(|e| e.id == id) else {
            continue;
        };
        if entry.display_name.is_empty() {
            continue;
        }
        let Some(pos) = world_to_screen(view_proj, world, screen_w, screen_h) else {
            continue;
        };
        let dist = (world - cam_eye).length().max(0.5);
        let font_size = (REF_SIZE * REF_DIST_M / dist).clamp(MIN_SIZE, MAX_SIZE);
        out.push(FloatingNameLabel {
            pos,
            name: entry.display_name.clone(),
            ally: false,
            font_size,
        });
    }
    out
}

fn world_to_screen(
    view_proj: glam::Mat4,
    world: glam::Vec3,
    screen_w: f32,
    screen_h: f32,
) -> Option<egui::Pos2> {
    let clip = view_proj * world.extend(1.0);
    if clip.w <= 1e-5 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !(-1.0..=1.0).contains(&ndc.x) || !(-1.0..=1.0).contains(&ndc.y) {
        return None;
    }
    let x = (ndc.x * 0.5 + 0.5) * screen_w;
    let y = (1.0 - (ndc.y * 0.5 + 0.5)) * screen_h;
    Some(egui::pos2(x, y))
}
