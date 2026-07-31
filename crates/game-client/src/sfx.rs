//! Client one-shot SFX (Web Audio). Present only — not sim.

use game_sim::LocomotionMode;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioBuffer, AudioContext};

use crate::pack;

const WALK_STEP_GAIN: f32 = 0.12;
const SPRINT_STEP_GAIN: f32 = 0.28;
const LAND_STEP_GAIN: f32 = 0.35;

pub struct Sfx {
    ctx: AudioContext,
    bang: AudioBuffer,
    gravel_steps: [AudioBuffer; 3],
    foot_prev: Option<f32>,
    last_gravel: u8,
    was_air: bool,
}

impl Sfx {
    pub async fn load() -> Result<Self, JsValue> {
        let ctx = AudioContext::new()?;
        let pack = pack::load_pack("sfx").await?;
        let bang = decode_wav(
            &ctx,
            pack.get("bang.wav").map_err(|e| JsValue::from_str(&e))?,
        )
        .await?;
        let s1 = decode_wav(
            &ctx,
            pack.get("step1.wav").map_err(|e| JsValue::from_str(&e))?,
        )
        .await?;
        let s2 = decode_wav(
            &ctx,
            pack.get("step2.wav").map_err(|e| JsValue::from_str(&e))?,
        )
        .await?;
        let s3 = decode_wav(
            &ctx,
            pack.get("step3.wav").map_err(|e| JsValue::from_str(&e))?,
        )
        .await?;
        Ok(Self {
            ctx,
            bang,
            gravel_steps: [s1, s2, s3],
            foot_prev: None,
            last_gravel: 0,
            was_air: false,
        })
    }

    pub fn resume(&self) {
        let _ = self.ctx.resume();
    }

    pub fn play_bang(&self) {
        self.play_buf(&self.bang, 1.0);
    }

    pub fn note_footsteps(&mut self, loco: LocomotionMode, phase: f32) {
        let air = loco.is_air();
        if self.was_air && !air {
            self.play_gravel_step(LAND_STEP_GAIN);
        }
        self.was_air = air;

        let active = matches!(
            loco,
            LocomotionMode::Walk | LocomotionMode::Sprint | LocomotionMode::Stopping
        );
        if !active {
            self.foot_prev = None;
            return;
        }
        let Some(prev) = self.foot_prev else {
            self.foot_prev = Some(phase);
            return;
        };
        let plants = foot_plants(prev, phase);
        self.foot_prev = Some(phase);
        let gain = if loco.is_sprint() {
            SPRINT_STEP_GAIN
        } else {
            WALK_STEP_GAIN
        };
        for _ in 0..plants {
            self.play_gravel_step(gain);
        }
    }

    fn play_gravel_step(&mut self, gain: f32) {
        let n = self.gravel_steps.len() as u8;
        let mut idx = (js_sys::Math::random() * f64::from(n)).floor() as u8;
        if idx >= n {
            idx = n - 1;
        }
        if idx == self.last_gravel {
            idx = (idx + 1) % n;
        }
        self.last_gravel = idx;
        self.play_buf(&self.gravel_steps[idx as usize], gain);
    }

    fn play_buf(&self, buf: &AudioBuffer, gain: f32) {
        let _ = self.ctx.resume();
        let Ok(src) = self.ctx.create_buffer_source() else {
            return;
        };
        src.set_buffer(Some(buf));
        let Ok(g) = self.ctx.create_gain() else {
            return;
        };
        g.gain().set_value(gain);
        if src.connect_with_audio_node(&g).is_err() {
            return;
        }
        if g.connect_with_audio_node(&self.ctx.destination()).is_err() {
            return;
        }
        let _ = src.start();
    }
}

fn foot_plants(prev: f32, curr: f32) -> u32 {
    if curr + 0.25 < prev {
        return 1;
    }
    let a = (prev * 2.0).floor();
    let b = (curr * 2.0).floor();
    if b > a {
        (b - a) as u32
    } else {
        0
    }
}

async fn decode_wav(ctx: &AudioContext, bytes: &[u8]) -> Result<AudioBuffer, JsValue> {
    let ab = js_sys::ArrayBuffer::new(bytes.len() as u32);
    js_sys::Uint8Array::new(&ab).copy_from(bytes);
    let promise = ctx.decode_audio_data(&ab)?;
    let decoded = JsFuture::from(promise).await?;
    decoded
        .dyn_into::<AudioBuffer>()
        .map_err(|_| JsValue::from_str("decodeAudioData: not AudioBuffer"))
}

#[derive(Default)]
pub enum SfxState {
    #[default]
    Idle,
    Loading,
    Ready(Sfx),
    Failed,
}

impl SfxState {
    pub fn resume(&self) {
        if let Self::Ready(sfx) = self {
            sfx.resume();
        }
    }

    pub fn play_bang(&self) {
        if let Self::Ready(sfx) = self {
            sfx.play_bang();
        }
    }

    pub fn note_footsteps(&mut self, loco: LocomotionMode, phase: f32) {
        if let Self::Ready(sfx) = self {
            sfx.note_footsteps(loco, phase);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::foot_plants;

    #[test]
    fn crosses_half() {
        assert_eq!(foot_plants(0.49, 0.51), 1);
        assert_eq!(foot_plants(0.1, 0.2), 0);
    }

    #[test]
    fn crosses_wrap() {
        assert_eq!(foot_plants(0.95, 0.05), 1);
    }
}
