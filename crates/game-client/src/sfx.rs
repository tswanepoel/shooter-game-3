//! Client one-shot SFX (Web Audio). Present only — not sim.

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioBuffer, AudioContext};

use crate::pack;

pub struct Sfx {
    ctx: AudioContext,
    bang: AudioBuffer,
}

impl Sfx {
    pub async fn load() -> Result<Self, JsValue> {
        let ctx = AudioContext::new()?;
        let pack = pack::load_pack("sfx").await?;
        let bytes = pack.get("bang.wav").map_err(|e| JsValue::from_str(&e))?;
        let bang = decode_wav(&ctx, bytes).await?;
        Ok(Self { ctx, bang })
    }

    pub fn resume(&self) {
        let _ = self.ctx.resume();
    }

    pub fn play_bang(&self) {
        let _ = self.ctx.resume();
        let Ok(src) = self.ctx.create_buffer_source() else {
            return;
        };
        src.set_buffer(Some(&self.bang));
        if src
            .connect_with_audio_node(&self.ctx.destination())
            .is_err()
        {
            return;
        }
        let _ = src.start();
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
}
