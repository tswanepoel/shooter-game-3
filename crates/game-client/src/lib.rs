//! WebGPU client: surface init, blank clear, and WASM-owned render loop.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/// Blank clear color (near-black, slightly tinted for a live GPU clear).
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.05,
    g: 0.06,
    b: 0.08,
    a: 1.0,
};

type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl Renderer {
    async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let width = canvas.client_width().max(1) as u32;
        let height = canvas.client_height().max(1) as u32;
        canvas.set_width(width);
        canvas.set_height(height);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {e}")))?;

        // SAFETY: wgpu copies the canvas handle; the surface is valid for the page lifetime.
        let surface: wgpu::Surface<'static> = unsafe { std::mem::transmute(surface) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| JsValue::from_str("No WebGPU adapter available"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("game-client-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create device: {e}")))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
        })
    }

    fn resize_if_needed(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.config.width == width && self.config.height == height {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&mut self) -> Result<(), JsValue> {
        let frame = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire swapchain texture: {e}")))?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

/// WebGPU renderer bound to a canvas, exposed to JS.
#[wasm_bindgen]
pub struct GameClient {
    renderer: Renderer,
    canvas: HtmlCanvasElement,
}

#[wasm_bindgen]
impl GameClient {
    /// Create a client on `canvas` and initialize WebGPU.
    #[wasm_bindgen(js_name = create)]
    pub async fn create(canvas: HtmlCanvasElement) -> Result<GameClient, JsValue> {
        console_error_panic_hook::set_once();

        let renderer = Renderer::new(canvas.clone()).await?;
        web_sys::console::log_1(&"WebGPU initialized; blank canvas ready".into());
        Ok(GameClient { renderer, canvas })
    }

    /// Clear and present one frame.
    #[wasm_bindgen(js_name = renderFrame)]
    pub fn render_frame(&mut self) -> Result<(), JsValue> {
        let width = self.canvas.client_width().max(1) as u32;
        let height = self.canvas.client_height().max(1) as u32;
        if self.canvas.width() != width || self.canvas.height() != height {
            self.canvas.set_width(width);
            self.canvas.set_height(height);
        }
        self.renderer.resize_if_needed(width, height);
        self.renderer.render()
    }

    /// Run the frame loop via `requestAnimationFrame`.
    #[wasm_bindgen(js_name = startRenderLoop)]
    pub fn start_render_loop(self) -> Result<(), JsValue> {
        let client = Rc::new(RefCell::new(self));
        let frame_cb: FrameCallback = Rc::new(RefCell::new(None));
        let frame_cb_clone = frame_cb.clone();

        *frame_cb.borrow_mut() = Some(Closure::new(move || {
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

        // Retain the rAF callback for the page lifetime.
        std::mem::forget(frame_cb);
        Ok(())
    }
}
