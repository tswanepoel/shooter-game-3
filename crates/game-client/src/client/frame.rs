//! Per-frame simulation, combat, and draw for ClientInner.

use std::collections::HashMap;

use game_net::PlayerId;
use game_sim::{FireState, HitBodyPart, PlayerHealth, ProjectileWorld, SelfState};
use wasm_bindgen::prelude::*;

#[cfg(feature = "debug-tools")]
use crate::lineup::LineupState;
use crate::mp;
use crate::renderer::canvas_buffer_size;
use crate::self_present::SelfPresentState;
use crate::ui_overlay::{DebugDraw, OverlayGpu};
use crate::view::{overview_view_matrix, LOOK_SENS_RAD_PER_PX};

use super::impact::apply_impact_in_present;
#[cfg(feature = "debug-tools")]
use super::load::capture_canvas_png;
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

        let mp_blocks_play = self.mp.blocks_play();
        let effects = self.mp.drain_frame_effects();
        if effects.release_pointer_lock && self.session.is_active() {
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                doc.exit_pointer_lock();
            }
        }
        if let Some(spawn) = effects.pending_spawn {
            self.self_state = SelfState::default_loadout();
            self.self_state.position = spawn.position;
            self.self_state.ocular_yaw = spawn.yaw;
            self.self_state.torso_yaw = spawn.yaw;
            self.fire = FireState::new();
            self.projectiles = ProjectileWorld::new();
            self.health_by_id.clear();
            self.ui.set_status(String::new());
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

        // Fly sync runs *after* mounted look + posed eye so F8 seeds at the true FP camera.
        #[cfg(feature = "debug-tools")]
        let want_fly = self.debug.flycam_wanted();
        #[cfg(feature = "debug-tools")]
        let was_fly = self.view.is_flycam();
        #[cfg(not(feature = "debug-tools"))]
        let was_fly = false;

        let play_ok = session_ok && !console_open && !was_fly && !mp_blocks_play;
        let mut fire_held = play_ok && self.move_input.fire_held();
        let emote_press = self.move_input.take_emote_press();
        let emote_release = self.move_input.take_emote_release();

        if play_ok {
            // Emote wheel (039): B open / release commit; look freezes into select.
            // Fire while open closes without commit (fire path may still run).
            if fire_held && self.emote_wheel.is_open() {
                self.emote_wheel.close();
            }
            if emote_press
                && !self.emote_wheel.is_open()
                && self.self_state.is_grounded()
                && !self.fire.blocks_weapon_side()
            {
                self.emote_wheel.open();
            }
            if self.emote_wheel.is_open() {
                self.emote_wheel.add_select_px(look.x, look.y);
                if emote_release || !self.move_input.emote_held() {
                    let slot = self.emote_wheel.highlighted_slot();
                    self.emote_wheel.close();
                    if let Some(id) = slot {
                        let _ = self
                            .self_state
                            .try_commit_emote(id, self.fire.blocks_weapon_side());
                    }
                }
            }

            let wheel_open = self.emote_wheel.is_open();
            // Don't fire while picking an emote (LMB already closed the wheel above).
            if wheel_open {
                fire_held = false;
            }
            if !wheel_open {
                self.self_state.apply_look(
                    dt,
                    -look.x * LOOK_SENS_RAD_PER_PX,
                    -look.y * LOOK_SENS_RAD_PER_PX,
                );
            }
            let (fwd, strafe) = self.move_input.axes();
            let mut sprint_tap = self.move_input.take_sprint();
            let jump = self.move_input.take_jump();
            let weapon_steps = self.move_input.take_weapon_cycle();
            let wdir = weapon_steps.signum();

            // Burst holds weapon-side actions (sprint, loadout wheel) â€” 038.
            // Emote radial open also blocks move/sprint/swap commit paths lightly:
            // freeze wish while open so a twitch does not cancel mid-pick.
            if self.fire.blocks_weapon_side() || wheel_open {
                sprint_tap = false;
            }

            let (fwd, strafe) = if wheel_open {
                (0.0, 0.0)
            } else {
                (fwd, strafe)
            };

            self.self_state.wish_forward = fwd.clamp(-1.0, 1.0);
            self.self_state.wish_strafe = strafe.clamp(-1.0, 1.0);
            if jump && !wheel_open {
                self.self_state.try_jump();
            }
            if !self.fire.blocks_weapon_side() && !wheel_open {
                for _ in 0..weapon_steps.unsigned_abs() {
                    self.self_state.cycle_weapon(wdir);
                    if let Some(letter) = self.self_state.active_blaster() {
                        self.fire.pay_ready(letter);
                    } else {
                        self.fire.sync_active_letter(None);
                    }
                }
            }
            self.self_state.apply_move(dt, fwd, strafe, sprint_tap);
        } else {
            if !play_ok {
                self.move_input.clear_keys();
                self.move_input.set_fire_held(false);
                self.move_input.set_emote_held(false);
                self.emote_wheel.close();
            }
            self.self_state.apply_move(dt, 0.0, 0.0, false);
        }

        self.self_state.tick_emote(dt);

        if fire_held && self.self_state.is_emoting() {
            self.self_state.clear_emote();
        }
        let owner = self.mp.player_id().unwrap_or(0);
        let look_origin = match &self.self_present {
            SelfPresentState::Ready(gpu) => gpu.view.look_origin,
            _ => self.self_state.position + glam::Vec3::new(0.0, 1.52, 0.27),
        };
        let muzzle_worlds = match &self.self_present {
            SelfPresentState::Ready(gpu) => gpu.fire_muzzle_worlds(&self.self_state),
            _ => Vec::new(),
        };
        let discharges = self.fire.tick(
            dt,
            &mut self.self_state,
            fire_held,
            owner,
            look_origin,
            &muzzle_worlds,
        );
        let mut claimed: Vec<game_sim::Projectile> = Vec::new();
        for d in &discharges {
            for p in &d.projectiles {
                claimed.push(p.clone());
                self.projectiles.spawn(p.clone());
            }
            let seed_pts = match &self.self_present {
                SelfPresentState::Ready(gpu) => {
                    gpu.flash_muzzle_worlds(&self.self_state, &d.fired_muzzles)
                }
                _ => muzzle_worlds.clone(),
            };
            self.renderer
                .fire_fx
                .note_self_discharge(&d.fired_muzzles, &seed_pts);
        }

        if !claimed.is_empty() {
            self.mp.claim_projectiles(&claimed);
        }

        // Accept peer projectiles (claim-and-relay).
        for batch in self.mp.take_peer_projectiles() {
            let mut muzzle_indices = Vec::new();
            let mut seed_pts = Vec::new();
            let mut weapon = b'p';
            for n in &batch.projectiles {
                if let Some(p) = mp::net_spawn_to_projectile(batch.id, n) {
                    weapon = p.weapon;
                    if !muzzle_indices.contains(&p.muzzle_index) {
                        muzzle_indices.push(p.muzzle_index);
                        seed_pts.push(p.origin);
                    }
                    self.projectiles.spawn(p);
                }
            }
            if !muzzle_indices.is_empty() {
                self.renderer.fire_fx.note_peer_projectiles(
                    batch.id,
                    weapon,
                    &muzzle_indices,
                    &seed_pts,
                );
            }
        }

        let local_id = self.mp.player_id();
        let firer_id = local_id.unwrap_or(0);
        if !self.mp.in_room() {
            self.health_by_id.clear();
        }

        let remote_samples: Vec<(PlayerId, game_net::DriveView)> = if self.mp.in_room() {
            self.mp
                .remotes()
                .samples()
                .map(|(id, s)| (id, s.drive.clone()))
                .collect()
        } else {
            Vec::new()
        };

        if let Some(id) = local_id {
            self.health_by_id
                .entry(id)
                .or_insert_with(|| PlayerHealth::read_from_self(&self.self_state));
        }
        for (id, _) in &remote_samples {
            self.health_by_id
                .entry(*id)
                .or_insert_with(PlayerHealth::full);
        }
        if self.mp.in_room() {
            let mut keep: HashMap<PlayerId, ()> =
                remote_samples.iter().map(|(id, _)| (*id, ())).collect();
            if let Some(id) = local_id {
                keep.insert(id, ());
            }
            self.health_by_id.retain(|id, _| keep.contains_key(id));
        }

        let remote_hit_states: Vec<(PlayerId, SelfState)> = remote_samples
            .iter()
            .filter_map(|(id, drive)| {
                if !self.mp.peer_living(*id) {
                    return None;
                }
                let h = self.health_by_id.get(id)?;
                if !h.alive {
                    return None;
                }
                let mut state = mp::drive_to_state(drive);
                state.alive = true;
                state.die_age_s = 0.0;
                Some((*id, state))
            })
            .collect();

        let hits = {
            let remote_present = &self.remote_present;
            let states = &remote_hit_states;
            self.projectiles.tick_hits_with(dt, firer_id, |from, to| {
                let mut best: Option<(f32, PlayerId, glam::Vec3, HitBodyPart)> = None;
                for (id, state) in states {
                    let Some(hit) = remote_present.trace_segment(*id, state, from, to) else {
                        continue;
                    };
                    let Some(part) = HitBodyPart::from_kit_name(&hit.part) else {
                        continue;
                    };
                    if best.map(|(bt, _, _, _)| hit.t < bt).unwrap_or(true) {
                        best = Some((hit.t, *id, hit.position, part));
                    }
                }
                best.map(|(_, id, p, part)| (id, p, part))
            })
        };
        for h in &hits {
            let dmg = apply_impact_in_present(
                h.target_id,
                h.ammo,
                h.speed,
                h.part,
                local_id,
                &mut self.self_state,
                &mut self.health_by_id,
            );
            if dmg > 0.0 {
                self.fire.add_hit_impulse(&mut self.self_state, dmg);
            }
        }
        if !hits.is_empty() {
            self.mp.claim_hits(&hits);
            // One flash per claim batch; successive frames re-pulse (044).
            self.hit_marker.pulse();
        }

        self.hit_marker.tick(dt);

        for batch in self.mp.take_peer_hits() {
            let Some(ammo) = mp::ammo_kind_from_wire(batch.hit.ammo) else {
                continue;
            };
            let Some(part) = HitBodyPart::from_wire(batch.hit.part) else {
                continue;
            };
            let dmg = apply_impact_in_present(
                batch.hit.target,
                ammo,
                batch.hit.speed,
                part,
                local_id,
                &mut self.self_state,
                &mut self.health_by_id,
            );
            if dmg > 0.0 {
                self.fire.add_hit_impulse(&mut self.self_state, dmg);
            }
        }

        self.self_state.tick_health(dt);
        if let Some(id) = local_id {
            self.health_by_id
                .insert(id, PlayerHealth::read_from_self(&self.self_state));
        }
        for (id, h) in self.health_by_id.iter_mut() {
            if local_id == Some(*id) {
                continue;
            }
            h.tick_regen(dt);
        }

        self.renderer.fire_fx.tick(dt);

        self.mp.on_frame(dt, &self.self_state);

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
                // Enter frame already baked this look into self â†’ fly pose; don't double-apply.
                let look = if was_fly { look } else { glam::Vec2::ZERO };
                self.view.update_flycam(dt, &self.fly_input, look);
            } else if console_open || !session_ok {
                self.fly_input.clear_keys();
            }
        }

        #[cfg(feature = "debug-tools")]
        let flycam = self.view.is_flycam();
        #[cfg(not(feature = "debug-tools"))]
        let flycam = false;

        let (cam_eye, cam_fwd) = self.view.eye_and_forward(&self.self_state);
        let view_mat = if mp_blocks_play {
            overview_view_matrix()
        } else {
            self.view.view_matrix(&self.self_state)
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
                            "fF {:.2}Â°  fT {:.2}Â°  fall {:.0}ms{}",
                            self.self_state.fire_fold_total().to_degrees(),
                            self.self_state.shoulder_fire_twist.to_degrees(),
                            fall_ms,
                            if cont { "  cont" } else { "" },
                        ));
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

            let (full, actions) = self
                .ui
                .run(raw, ppp, phase, &roster, connecting, debug_draw);

            if let Some((room, name)) = actions.join {
                self.ui.set_status("joiningâ€¦");
                self.mp.begin_join_with(&room, &name);
            }
            if actions.spawn {
                self.mp.request_spawn();
            }
            if actions.leave {
                self.mp.leave();
                self.remote_present.clear();
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
