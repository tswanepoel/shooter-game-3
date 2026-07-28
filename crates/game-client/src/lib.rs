//! WebGPU client: mounted self view, input session, optional debug tools.

#[cfg(target_arch = "wasm32")]
mod body_hit;
#[cfg(target_arch = "wasm32")]
mod client;
#[cfg(target_arch = "wasm32")]
mod corpse_present;
#[cfg(all(target_arch = "wasm32", feature = "debug-tools"))]
mod debug;
#[cfg(target_arch = "wasm32")]
mod emote_wheel;
#[cfg(target_arch = "wasm32")]
mod fire_fx;
#[cfg(target_arch = "wasm32")]
mod hit_marker;
#[cfg(target_arch = "wasm32")]
mod input;
#[cfg(all(target_arch = "wasm32", feature = "debug-tools"))]
mod lineup;
#[cfg(target_arch = "wasm32")]
mod mesh;
#[cfg(target_arch = "wasm32")]
mod mp;
#[cfg(target_arch = "wasm32")]
mod pack;
#[cfg(target_arch = "wasm32")]
mod preferences;
#[cfg(target_arch = "wasm32")]
mod remote_present;
#[cfg(target_arch = "wasm32")]
mod renderer;
#[cfg(target_arch = "wasm32")]
mod reticle;
#[cfg(target_arch = "wasm32")]
mod self_present;
#[cfg(target_arch = "wasm32")]
mod ui_overlay;
#[cfg(target_arch = "wasm32")]
mod view;
#[cfg(target_arch = "wasm32")]
mod world_loot;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(dead_code)]
mod native_tests;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use game_sim::SelfState;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

#[cfg(all(target_arch = "wasm32", feature = "debug-tools"))]
use client::load::maybe_kick_lineup_load;
#[cfg(target_arch = "wasm32")]
use client::load::{maybe_kick_corpse_loads, maybe_kick_remote_loads, maybe_kick_self_load};
#[cfg(target_arch = "wasm32")]
pub(crate) use client::ClientInner;
#[cfg(all(target_arch = "wasm32", feature = "debug-tools"))]
use debug::{DebugHost, DebugTools};
#[cfg(target_arch = "wasm32")]
use input::install_input_handlers;
#[cfg(target_arch = "wasm32")]
use renderer::{canvas_buffer_size, Renderer};
#[cfg(target_arch = "wasm32")]
use ui_overlay::UiOverlay;
#[cfg(target_arch = "wasm32")]
use view::ViewController;

#[cfg(target_arch = "wasm32")]
type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct GameClient {
    inner: Rc<RefCell<ClientInner>>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl GameClient {
    #[wasm_bindgen(js_name = create)]
    pub async fn create(canvas: HtmlCanvasElement) -> Result<GameClient, JsValue> {
        console_error_panic_hook::set_once();

        // WebGPU is SecureContext-only; `http://LAN-IP` fails (localhost HTTP is fine).
        if let Some(window) = web_sys::window() {
            if !window.is_secure_context() {
                return Err(JsValue::from_str(
                    "WebGPU requires a secure context. Open via https:// (npm run dev:lan) \
                     or http://localhost — not plain http:// on a LAN IP.",
                ));
            }
        }

        let renderer = Renderer::new(canvas.clone()).await?;
        let self_state = SelfState::default_loadout();
        let view = ViewController::new();
        renderer.write_view_proj(view.view_matrix());

        #[cfg(feature = "debug-tools")]
        let debug = DebugTools::new();
        let ui = UiOverlay::new(&renderer.device, renderer.config.format);

        let ppp = canvas_buffer_size(&canvas, renderer.max_texture_dim).2;
        let inner = Rc::new(RefCell::new(ClientInner::new(
            renderer,
            canvas,
            ppp,
            self_state,
            view,
            ui,
            #[cfg(feature = "debug-tools")]
            debug,
        )));

        web_sys::console::log_1(
            &"WebGPU initialized; self mount ready (click canvas to capture input)".into(),
        );
        Ok(GameClient { inner })
    }

    #[cfg(feature = "debug-tools")]
    #[wasm_bindgen(js_name = debugHost)]
    pub fn debug_host(&self) -> DebugHost {
        DebugHost::new(self.inner.clone())
    }

    #[wasm_bindgen(js_name = renderFrame)]
    pub fn render_frame(&mut self) -> Result<(), JsValue> {
        self.inner.borrow_mut().render_frame()
    }

    #[wasm_bindgen(js_name = startRenderLoop)]
    pub fn start_render_loop(&self) -> Result<(), JsValue> {
        let canvas = self.inner.borrow().canvas.clone();
        install_input_handlers(self.inner.clone(), &canvas);

        let client = self.inner.clone();
        let frame_cb: FrameCallback = Rc::new(RefCell::new(None));
        let frame_cb_clone = frame_cb.clone();

        *frame_cb.borrow_mut() = Some(Closure::new(move || {
            maybe_kick_self_load(&client);
            maybe_kick_remote_loads(&client);
            maybe_kick_corpse_loads(&client);
            #[cfg(feature = "debug-tools")]
            maybe_kick_lineup_load(&client);

            {
                let mut client = client.borrow_mut();
                if let Err(err) = client.render_frame() {
                    web_sys::console::error_1(&err);
                }
            }

            if let Some(window) = web_sys::window() {
                let cb = frame_cb_clone.borrow();
                if let Some(closure) = cb.as_ref() {
                    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
                }
            }
        }));

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window available"))?;
        {
            let cb = frame_cb.borrow();
            let closure = cb
                .as_ref()
                .ok_or_else(|| JsValue::from_str("Render loop closure missing"))?;
            window
                .request_animation_frame(closure.as_ref().unchecked_ref())
                .map_err(|e| JsValue::from_str(&format!("Failed to schedule frame: {e:?}")))?;
        }

        std::mem::forget(frame_cb);
        Ok(())
    }
}
