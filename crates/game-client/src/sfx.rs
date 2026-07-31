//! Client one-shot SFX (Web Audio). Present only — not sim.

use game_sim::LocomotionMode;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioBuffer, AudioContext};

use crate::map_present::FootKind;
use crate::pack;

const WALK_STEP_GAIN: f32 = 0.12;
const SPRINT_STEP_GAIN: f32 = 0.28;
const LAND_STEP_GAIN: f32 = 0.2;
/// Second sole on jump/fall land — parallel plant, randomized stagger.
const LAND_STEP_OFFSET_MIN_S: f64 = 0.015;
const LAND_STEP_OFFSET_MAX_S: f64 = 0.05;

pub struct Sfx {
    ctx: AudioContext,
    bang: AudioBuffer,
    gravel_steps: [AudioBuffer; 3],
    cement_steps: [AudioBuffer; 3],
    foot_prev: Option<f32>,
    last_gravel: u8,
    last_cement: u8,
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
        let gravel_steps = [
            decode_wav(
                &ctx,
                pack.get("gravel-step1.wav")
                    .map_err(|e| JsValue::from_str(&e))?,
            )
            .await?,
            decode_wav(
                &ctx,
                pack.get("gravel-step2.wav")
                    .map_err(|e| JsValue::from_str(&e))?,
            )
            .await?,
            decode_wav(
                &ctx,
                pack.get("gravel-step3.wav")
                    .map_err(|e| JsValue::from_str(&e))?,
            )
            .await?,
        ];
        let cement_steps = [
            decode_wav(
                &ctx,
                pack.get("cement-step1.wav")
                    .map_err(|e| JsValue::from_str(&e))?,
            )
            .await?,
            decode_wav(
                &ctx,
                pack.get("cement-step2.wav")
                    .map_err(|e| JsValue::from_str(&e))?,
            )
            .await?,
            decode_wav(
                &ctx,
                pack.get("cement-step3.wav")
                    .map_err(|e| JsValue::from_str(&e))?,
            )
            .await?,
        ];
        Ok(Self {
            ctx,
            bang,
            gravel_steps,
            cement_steps,
            foot_prev: None,
            last_gravel: 0,
            last_cement: 0,
            was_air: false,
        })
    }

    pub fn resume(&self) {
        let _ = self.ctx.resume();
    }

    pub fn play_bang(&self) {
        self.play_buf(&self.bang, 1.0, 0.0);
    }

    pub fn note_footsteps(&mut self, loco: LocomotionMode, phase: f32, surface: FootKind) {
        let air = loco.is_air();
        if self.was_air && !air {
            // Two soles land nearly together — distinct variants, randomized stagger.
            let stagger = LAND_STEP_OFFSET_MIN_S
                + js_sys::Math::random() * (LAND_STEP_OFFSET_MAX_S - LAND_STEP_OFFSET_MIN_S);
            self.play_step(surface, LAND_STEP_GAIN, 0.0);
            self.play_step(surface, LAND_STEP_GAIN, stagger);
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
            self.play_step(surface, gain, 0.0);
        }
    }

    fn play_step(&mut self, surface: FootKind, gain: f32, when_s: f64) {
        match surface {
            FootKind::Gravel => {
                let idx = pick_variant(self.gravel_steps.len() as u8, &mut self.last_gravel);
                self.play_buf(&self.gravel_steps[idx as usize], gain, when_s);
            }
            FootKind::Cement => {
                let idx = pick_variant(self.cement_steps.len() as u8, &mut self.last_cement);
                self.play_buf(&self.cement_steps[idx as usize], gain, when_s);
            }
        }
    }

    fn play_buf(&self, buf: &AudioBuffer, gain: f32, when_s: f64) {
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
        let when = self.ctx.current_time() + when_s;
        let _ = src.start_with_when(when);
    }
}

fn pick_variant(n: u8, last: &mut u8) -> u8 {
    let mut idx = (js_sys::Math::random() * f64::from(n)).floor() as u8;
    if idx >= n {
        idx = n - 1;
    }
    if idx == *last {
        idx = (idx + 1) % n;
    }
    *last = idx;
    idx
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

    pub fn note_footsteps(&mut self, loco: LocomotionMode, phase: f32, surface: FootKind) {
        if let Self::Ready(sfx) = self {
            sfx.note_footsteps(loco, phase, surface);
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
