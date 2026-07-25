//! Play-session controls: look, loco, emote wheel, weapon cycle.

use crate::view::LOOK_SENS_RAD_PER_PX;

use super::super::ClientInner;

impl ClientInner {
    /// Apply loco, look, emote, and weapon cycle when play is allowed.
    /// Returns whether fire is held for the combat stage.
    pub(super) fn tick_play_controls(&mut self, dt: f32, look: glam::Vec2, play_ok: bool) -> bool {
        let mut fire_held = play_ok && self.move_input.fire_held();
        let emote_press = self.move_input.take_emote_press();
        let emote_release = self.move_input.take_emote_release();

        #[cfg(feature = "debug-tools")]
        {
            self.lookhud_pre_f_deg = self.self_state.facing.to_degrees();
            self.lookhud_a_yaw_deg = 0.0;
            self.lookhud_a_pitch_deg = 0.0;
            self.lookhud_did_apply = false;
        }

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
                let a_yaw = -look.x * LOOK_SENS_RAD_PER_PX;
                let a_pitch = -look.y * LOOK_SENS_RAD_PER_PX;
                self.self_state.apply_look(dt, a_yaw, a_pitch);
                #[cfg(feature = "debug-tools")]
                {
                    self.lookhud_a_yaw_deg = a_yaw.to_degrees();
                    self.lookhud_a_pitch_deg = a_pitch.to_degrees();
                    self.lookhud_did_apply = true;
                    self.lookhud_why = "ok";
                }
            } else {
                #[cfg(feature = "debug-tools")]
                {
                    self.lookhud_why = "wheel";
                }
            }
            let (fwd, strafe) = self.move_input.axes();
            let mut sprint_tap = self.move_input.take_sprint();
            let jump = self.move_input.take_jump();
            let weapon_steps = self.move_input.take_weapon_cycle();
            let wdir = weapon_steps.signum();

            // Burst holds weapon-side actions (sprint, loadout wheel) — 038.
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

        #[cfg(feature = "debug-tools")]
        {
            self.lookhud_post_f_deg = self.self_state.facing.to_degrees();
            if look.x != 0.0 {
                self.lookhud_sum_dx += look.x;
                self.lookhud_sum_a_yaw_deg += self.lookhud_a_yaw_deg;
            } else {
                self.lookhud_sum_dx = 0.0;
                self.lookhud_sum_a_yaw_deg = 0.0;
            }
        }

        fire_held
    }
}
