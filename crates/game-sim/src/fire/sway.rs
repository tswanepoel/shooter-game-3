//! Multi-band resting sway (breath + tremor + mean-reverting drift).

use crate::weapons::WeaponSway;

#[derive(Debug, Clone)]
pub(crate) struct SwayState {
    t: f32,
    pub(crate) pitch_rad: f32,
    pub(crate) yaw_rad: f32,
    drift_pitch: f32,
    drift_yaw: f32,
    /// 0 = fully damped (hard look), 1 = full resting sway.
    damp: f32,
    last_yaw: f32,
    last_pitch: f32,
    has_look: bool,
    rng: u32,
}

impl SwayState {
    pub(crate) fn new(rng: u32) -> Self {
        Self {
            t: 0.0,
            pitch_rad: 0.0,
            yaw_rad: 0.0,
            drift_pitch: 0.0,
            drift_yaw: 0.0,
            damp: 1.0,
            last_yaw: 0.0,
            last_pitch: 0.0,
            has_look: false,
            rng,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.pitch_rad = 0.0;
        self.yaw_rad = 0.0;
        self.drift_pitch = 0.0;
        self.drift_yaw = 0.0;
        self.damp = 1.0;
        self.has_look = false;
    }

    fn next_gauss(&mut self) -> f32 {
        // Box–Muller from two xorshift samples.
        let u1 = self.next_rand01().max(1e-7);
        let u2 = self.next_rand01();
        (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
    }

    fn next_rand01(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32) / (u32::MAX as f32)
    }

    pub(crate) fn advance(&mut self, dt: f32, params: WeaponSway, look_yaw: f32, look_pitch: f32) {
        let dt = dt.max(0.0);
        if dt <= 0.0 {
            return;
        }

        // Look-rate damp: hard tracking quiets sway; still eases back in.
        let rate = if self.has_look {
            let dy = look_yaw - self.last_yaw;
            let dp = look_pitch - self.last_pitch;
            ((dy * dy + dp * dp).sqrt() / dt).max(0.0)
        } else {
            0.0
        };
        self.last_yaw = look_yaw;
        self.last_pitch = look_pitch;
        self.has_look = true;
        // ~1.2 rad/s full look → near-zero target.
        let target = 1.0 / (1.0 + (rate / 1.2).powi(2));
        let tau = if target < self.damp { 0.12 } else { 0.40 };
        let a = 1.0 - (-dt / tau).exp();
        self.damp += (target - self.damp) * a;

        self.t += dt;
        let t = self.t;
        let tau_d = params.drift_tau_s.max(1e-3);
        // Stationary std ≈ drift_amp: σ = amp * sqrt(2/τ).
        let sigma = params.drift_amp_deg.to_radians() * (2.0 / tau_d).sqrt();
        let noise_scale = sigma * dt.sqrt();
        self.drift_pitch += -self.drift_pitch / tau_d * dt + noise_scale * self.next_gauss();
        self.drift_yaw += -self.drift_yaw / tau_d * dt + noise_scale * self.next_gauss();

        let breath = params.breath_amp_deg.to_radians();
        let tremor = params.tremor_amp_deg.to_radians();
        let w_b = std::f32::consts::TAU * params.breath_hz;
        let w_t = std::f32::consts::TAU * params.tremor_hz;

        // Breath: mostly pitch; slight out-of-phase yaw.
        let breath_p = breath * (w_b * t).sin();
        let breath_y = breath * 0.28 * (w_b * 0.93 * t + 1.1).sin();
        // Tremor: soft micro-band, incommensurate axes.
        let tremor_p = tremor * (w_t * t).sin();
        let tremor_y = tremor * (w_t * 1.17 * t + 0.7).sin();

        let scale = self.damp;
        self.pitch_rad = (breath_p + tremor_p + self.drift_pitch) * scale;
        self.yaw_rad = (breath_y + tremor_y + self.drift_yaw) * scale;
    }
}
