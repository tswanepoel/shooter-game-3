//! Ground and air locomotion for SelfState.

use super::types::LocomotionMode;
use super::SelfState;
use crate::{MapWorld, STEP_DOWN_M, STEP_UP_M};

/// Walk speed on the ground plane (016).
/// Kenney `walk` at 1×: stance sole slip = \(2 L \sin\theta\) per half-cycle
/// (\(L = 2/3\,\mathrm{m}\), \(\theta = 60°\), \(T = 2/3\,\mathrm{s}\)) → \(2\sqrt{3}\) m/s.
pub const WALK_SPEED_M_S: f32 = 3.464_101_6; // 2√3
/// Kenney `walk` clip duration (seconds). Phase maps as `phase * duration`.
pub const WALK_CLIP_DURATION_S: f32 = 2.0 / 3.0;
/// Ground metres per full walk cycle (phase 0→1). At walk speed this plays the clip at 1×.
pub const WALK_STRIDE_M: f32 = WALK_SPEED_M_S * WALK_CLIP_DURATION_S;

/// Sprint speed on the ground plane (020). ~1.75× walk.
pub const SPRINT_SPEED_M_S: f32 = WALK_SPEED_M_S * 1.75;
/// Kenney `sprint` clip duration (seconds).
pub const SPRINT_CLIP_DURATION_S: f32 = 0.5;
/// Ground metres per full sprint cycle (phase 0→1). At sprint speed this plays the clip at 1×.
pub const SPRINT_STRIDE_M: f32 = SPRINT_SPEED_M_S * SPRINT_CLIP_DURATION_S;

/// Full stamina (0…1).
pub const STAMINA_MAX: f32 = 1.0;
/// Continuous sprint duration on a full bar (seconds).
pub const STAMINA_SPRINT_S: f32 = 4.0;
/// Full refill time while not sprinting (seconds).
pub const STAMINA_REGEN_S: f32 = 4.0;
/// Minimum fill required to *start* a sprint (avoid premature flicker).
pub const STAMINA_MIN_TO_START: f32 = 0.25;

pub const JUMP_PEAK_M: f32 = 1.1;
pub const JUMP_TIME_TO_APEX_S: f32 = 0.25;
pub const JUMP_GRAVITY_M_S2: f32 = 2.0 * JUMP_PEAK_M / (JUMP_TIME_TO_APEX_S * JUMP_TIME_TO_APEX_S);
pub const JUMP_LAUNCH_M_S: f32 = JUMP_GRAVITY_M_S2 * JUMP_TIME_TO_APEX_S;

impl SelfState {
    pub fn is_grounded(&self) -> bool {
        !self.locomotion.is_air()
    }

    /// Ground plane speed for the current locomotion (walk or sprint).
    pub fn ground_speed(&self) -> f32 {
        if self.locomotion.is_sprint() {
            SPRINT_SPEED_M_S
        } else {
            WALK_SPEED_M_S
        }
    }

    pub fn try_jump(&mut self) {
        if !self.alive || !self.is_grounded() {
            return;
        }
        self.clear_emote();
        let speed = self.ground_speed();
        let mut wish =
            self.look_forward_xz() * self.wish_forward + self.look_right_xz() * self.wish_strafe;
        wish.y = 0.0;
        if wish.length_squared() > 1e-12 {
            let dir = wish.normalize();
            self.air_vel_x = dir.x * speed;
            self.air_vel_z = dir.z * speed;
        } else {
            self.air_vel_x = 0.0;
            self.air_vel_z = 0.0;
        }
        self.velocity_y = JUMP_LAUNCH_M_S;
        self.locomotion = LocomotionMode::Air;
    }

    /// Look-relative walk wish (−1…1). Ground: phase from distance. Air: coast + gravity.
    /// `sprint_tap` is a Shift press edge (020); latches sprint only (no cancel). Stamina gates start/drain.
    pub fn apply_move(&mut self, dt: f32, forward: f32, strafe: f32, sprint_tap: bool) {
        self.apply_move_world(dt, forward, strafe, sprint_tap, &MapWorld::empty());
    }

    /// Same as [`Self::apply_move`] against map solids (066).
    pub fn apply_move_world(
        &mut self,
        dt: f32,
        forward: f32,
        strafe: f32,
        sprint_tap: bool,
        world: &MapWorld,
    ) {
        let dt = dt.max(0.0);
        if !self.alive {
            self.wish_forward = 0.0;
            self.wish_strafe = 0.0;
            self.sprint_latched = false;
            if self.locomotion.is_air() {
                self.integrate_air(dt, false, world);
            }
            self.sync_pose();
            return;
        }

        self.wish_forward = forward.clamp(-1.0, 1.0);
        self.wish_strafe = strafe.clamp(-1.0, 1.0);

        // Emote cancels on any walk wish or sprint engage (039).
        let wish_moving = self.wish_forward.abs() > 1e-6 || self.wish_strafe.abs() > 1e-6;
        if self.is_emoting() && (wish_moving || sprint_tap) {
            self.clear_emote();
        }

        // Shift tap only engages; never cancels (empty bar / stop / lose W clear the latch).
        if sprint_tap && !self.sprint_latched && self.stamina >= STAMINA_MIN_TO_START {
            self.sprint_latched = true;
        }

        let mut wish =
            self.look_forward_xz() * self.wish_forward + self.look_right_xz() * self.wish_strafe;
        wish.y = 0.0;

        let moving = wish.length_squared() > 1e-12;

        if self.locomotion.is_air() {
            self.integrate_air(dt, moving, world);
            // Latched sprint still costs stamina aloft (no jump-regen exploit).
            if self.sprint_latched {
                self.drain_stamina(dt);
            } else {
                self.regen_stamina(dt);
            }
        } else if moving {
            // Sprint is forward-only (W); A/D/S alone walk.
            let forward_ok = self.wish_forward > 1e-6;
            if self.sprint_latched && !forward_ok {
                self.sprint_latched = false;
            }
            let sprinting = self.sprint_latched && forward_ok && self.stamina > 0.0;

            let (speed, stride) = if sprinting {
                (SPRINT_SPEED_M_S, SPRINT_STRIDE_M)
            } else {
                (WALK_SPEED_M_S, WALK_STRIDE_M)
            };

            let dir = wish.normalize();
            let step = dir * (speed * dt);
            let fell = self.displace_ground(step.x, step.z, world);
            if fell {
                self.air_vel_x = dir.x * speed;
                self.air_vel_z = dir.z * speed;
                self.velocity_y = 0.0;
                self.locomotion = LocomotionMode::Air;
            } else {
                self.locomotion = if sprinting {
                    LocomotionMode::Sprint
                } else {
                    LocomotionMode::Walk
                };
                let dphase = if stride > 1e-8 {
                    speed * dt / stride
                } else {
                    0.0
                };
                self.walk_phase = (self.walk_phase + dphase).rem_euclid(1.0);
            }

            if self.sprint_latched {
                self.drain_stamina(dt);
                if !self.sprint_latched && !self.locomotion.is_air() {
                    self.locomotion = LocomotionMode::Walk;
                }
            } else {
                self.regen_stamina(dt);
            }
        } else {
            self.sprint_latched = false;
            // Stay snapped to support while idle on solids.
            let support = world.support_y(self.position.x, self.position.z);
            if self.position.y > support + STEP_DOWN_M {
                self.velocity_y = 0.0;
                self.air_vel_x = 0.0;
                self.air_vel_z = 0.0;
                self.locomotion = LocomotionMode::Air;
            } else {
                self.position.y = support;
                if self.locomotion.is_sprint() {
                    self.locomotion = LocomotionMode::Stand;
                    self.walk_phase = 0.0;
                } else {
                    let dphase = if WALK_STRIDE_M > 1e-8 {
                        WALK_SPEED_M_S * dt / WALK_STRIDE_M
                    } else {
                        0.0
                    };
                    self.settle_walk_stop(dphase);
                }
                self.regen_stamina(dt);
            }
        }

        self.sync_pose();
    }

    /// Ground XZ step. Returns true when the figure walked off a drop into air.
    fn displace_ground(&mut self, dx: f32, dz: f32, world: &MapWorld) -> bool {
        match self.probe_ground_step(dx, dz, world) {
            GroundStep::Moved { fell } => fell,
            GroundStep::Blocked => match self.probe_ground_step(dx, 0.0, world) {
                GroundStep::Moved { fell } => fell,
                GroundStep::Blocked => match self.probe_ground_step(0.0, dz, world) {
                    GroundStep::Moved { fell } => fell,
                    GroundStep::Blocked => false,
                },
            },
        }
    }

    fn probe_ground_step(&mut self, dx: f32, dz: f32, world: &MapWorld) -> GroundStep {
        let nx = self.position.x + dx;
        let nz = self.position.z + dz;
        let floor = world.support_y(nx, nz);
        let dy = floor - self.position.y;

        if dy > STEP_UP_M {
            return GroundStep::Blocked;
        }

        if dy < -STEP_DOWN_M {
            self.position.x = nx;
            self.position.z = nz;
            // Leave y; caller enters air.
            return GroundStep::Moved { fell: true };
        }

        self.position.x = nx;
        self.position.z = nz;
        self.position.y = floor;
        GroundStep::Moved { fell: false }
    }

    fn displace_air_xz(&mut self, dx: f32, dz: f32, world: &MapWorld) {
        if self.try_air_step(dx, dz, world) {
            return;
        }
        if self.try_air_step(dx, 0.0, world) {
            return;
        }
        let _ = self.try_air_step(0.0, dz, world);
    }

    fn try_air_step(&mut self, dx: f32, dz: f32, world: &MapWorld) -> bool {
        let nx = self.position.x + dx;
        let nz = self.position.z + dz;
        if world.inside_solid(nx, self.position.y, nz) {
            return false;
        }
        self.position.x = nx;
        self.position.z = nz;
        true
    }

    fn drain_stamina(&mut self, dt: f32) {
        self.stamina = (self.stamina - dt / STAMINA_SPRINT_S).max(0.0);
        if self.stamina <= 0.0 {
            self.stamina = 0.0;
            self.sprint_latched = false;
        }
    }

    fn regen_stamina(&mut self, dt: f32) {
        if self.stamina < STAMINA_MAX {
            self.stamina = (self.stamina + dt / STAMINA_REGEN_S).min(STAMINA_MAX);
        }
    }

    fn integrate_air(&mut self, dt: f32, land_wish: bool, world: &MapWorld) {
        let dx = self.air_vel_x * dt;
        let dz = self.air_vel_z * dt;
        self.displace_air_xz(dx, dz, world);

        self.velocity_y -= JUMP_GRAVITY_M_S2 * dt;
        self.position.y += self.velocity_y * dt;
        self.locomotion = LocomotionMode::Air;

        let floor = world.support_y(self.position.x, self.position.z);
        if self.position.y <= floor && self.velocity_y <= 0.0 {
            self.position.y = floor;
            self.velocity_y = 0.0;
            self.air_vel_x = 0.0;
            self.air_vel_z = 0.0;
            self.walk_phase = 0.0;
            self.locomotion = if land_wish {
                LocomotionMode::Walk
            } else {
                LocomotionMode::Stand
            };
        }
    }

    /// Finish walk in place to the nearest neutral, then stand.
    ///
    /// Kenney walk neutrals land at phase 0 and 0.5 (same rest). From the first
    /// half, 0.5 is the fastest back-out; from the second half, the cycle end.
    fn settle_walk_stop(&mut self, dphase: f32) {
        let settling = matches!(
            self.locomotion,
            LocomotionMode::Walk | LocomotionMode::Stopping
        ) && self.walk_phase > 1e-6
            && (self.walk_phase - 0.5).abs() > 1e-6;

        if !settling {
            self.locomotion = LocomotionMode::Stand;
            self.walk_phase = 0.0;
            return;
        }

        // Target: mid-cycle neutral if still in first half, else cycle end.
        let target = if self.walk_phase < 0.5 { 0.5 } else { 1.0 };
        let next = self.walk_phase + dphase;
        if next >= target - 1e-6 {
            self.locomotion = LocomotionMode::Stand;
            self.walk_phase = 0.0;
        } else {
            self.locomotion = LocomotionMode::Stopping;
            self.walk_phase = next;
        }
    }
}

enum GroundStep {
    Moved { fell: bool },
    Blocked,
}
