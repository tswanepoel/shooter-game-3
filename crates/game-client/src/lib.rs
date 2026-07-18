//! WebGPU client: mounted self view of an empty scene with a debug grid.
//! Optional debug flycam (feature `debug-tools`) unmounts for free inspection.

#![cfg(target_arch = "wasm32")]

#[cfg(feature = "debug-tools")]
mod debug;
mod view;

use std::cell::RefCell;
use std::rc::Rc;

use game_sim::{
    CAMERA_FAR_M, CAMERA_NEAR_M, CAMERA_VERTICAL_FOV_RAD, DEBUG_GRID_HALF_EXTENT_M,
    GRID_MAJOR_EVERY, GRID_MINOR_SPACING_M,
};
use glam::Mat4;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[cfg(feature = "debug-tools")]
use debug::{install_input_handlers, DebugHost, DebugTools};
#[cfg(feature = "debug-tools")]
use view::FlyInput;
use view::ViewController;

/// Scene clear colour (presentation only).
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.05,
    g: 0.06,
    b: 0.08,
    a: 1.0,
};

/// Soft debug grid colours (RGB + alpha). Alpha blends over the clear colour.
const MINOR_LINE_COLOR: [f32; 4] = [0.40, 0.45, 0.52, 0.28];
const MAJOR_LINE_COLOR: [f32; 4] = [0.65, 0.70, 0.78, 0.45];

const GRID_SHADER: &str = r#"
struct FrameUniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> frame: FrameUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = frame.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

type FrameCallback = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniforms {
    view_proj: [[f32; 4]; 4],
}

struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
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

        let depth_view = create_depth_view(&device, width, height);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("grid-shader"),
            source: wgpu::ShaderSource::Wgsl(GRID_SHADER.into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame-uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame-bind-group-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame-bind-group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                // Transparent lines: read depth, don't write (avoids self-order artifacts later).
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let grid = build_debug_grid();
        let vertex_count = grid.len() as u32;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid-vertices"),
            size: (grid.len() * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&grid));

        Ok(Self {
            surface,
            device,
            queue,
            config,
            depth_view,
            pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            vertex_count,
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
        self.depth_view = create_depth_view(&self.device, width, height);
    }

    fn write_view_proj(&self, view: Mat4) {
        let aspect = self.config.width as f32 / self.config.height as f32;
        let proj =
            perspective_rh_wgpu(CAMERA_VERTICAL_FOV_RAD, aspect, CAMERA_NEAR_M, CAMERA_FAR_M);
        let uniforms = FrameUniforms {
            view_proj: (proj * view).to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    fn render_scene(&mut self, draw_grid: bool) -> Result<wgpu::SurfaceTexture, JsValue> {
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
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if draw_grid {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.draw(0..self.vertex_count, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        // Return frame so caller can overlay egui then present.
        // We already submitted scene — egui needs another encoder.
        // Store view by re-creating from frame after return.
        Ok(frame)
    }
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    depth.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Right-handed perspective, clip depth 0..1 (WebGPU / wgpu / Vulkan).
fn perspective_rh_wgpu(fovy_rad: f32, aspect: f32, z_near: f32, z_far: f32) -> Mat4 {
    let h = 1.0 / (fovy_rad * 0.5).tan();
    let w = h / aspect;
    let r = z_far / (z_near - z_far);
    Mat4::from_cols(
        glam::Vec4::new(w, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, h, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, r, -1.0),
        glam::Vec4::new(0.0, 0.0, r * z_near, 0.0),
    )
}

/// World-space line list on y = 0 (client debug overlay only).
fn build_debug_grid() -> Vec<Vertex> {
    let half = DEBUG_GRID_HALF_EXTENT_M;
    let step = GRID_MINOR_SPACING_M;
    let major_every = GRID_MAJOR_EVERY as i32;
    let n = (half / step).round() as i32;

    let mut vertices = Vec::with_capacity(((n * 2 + 1) * 4) as usize);
    for i in -n..=n {
        let t = i as f32 * step;
        let color = if i.rem_euclid(major_every) == 0 {
            MAJOR_LINE_COLOR
        } else {
            MINOR_LINE_COLOR
        };

        vertices.push(Vertex {
            position: [-half, 0.0, t],
            color,
        });
        vertices.push(Vertex {
            position: [half, 0.0, t],
            color,
        });

        vertices.push(Vertex {
            position: [t, 0.0, -half],
            color,
        });
        vertices.push(Vertex {
            position: [t, 0.0, half],
            color,
        });
    }
    vertices
}

/// Shared client state (render loop + optional debug host).
pub(crate) struct ClientInner {
    renderer: Renderer,
    canvas: HtmlCanvasElement,
    pub(crate) view: ViewController,
    #[cfg(feature = "debug-tools")]
    pub(crate) debug: DebugTools,
    #[cfg(feature = "debug-tools")]
    pub(crate) fly_input: FlyInput,
    #[cfg(feature = "debug-tools")]
    last_frame_secs: f64,
}

impl ClientInner {
    /// egui points → framebuffer pixels.
    ///
    /// Canvas buffer is set to CSS client size (not `× devicePixelRatio`), so
    /// this is always 1.0. Do not use raw `devicePixelRatio` — that double-scales
    /// the UI. When we add true HiDPI buffers, set ppp = buffer/CSS here.
    #[cfg(feature = "debug-tools")]
    pub(crate) fn pixels_per_point(&self) -> f32 {
        1.0
    }

    fn render_frame(&mut self) -> Result<(), JsValue> {
        let width = self.canvas.client_width().max(1) as u32;
        let height = self.canvas.client_height().max(1) as u32;
        if self.canvas.width() != width || self.canvas.height() != height {
            self.canvas.set_width(width);
            self.canvas.set_height(height);
        }
        self.renderer.resize_if_needed(width, height);

        #[cfg(feature = "debug-tools")]
        {
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

            let want_fly = self.debug.flycam_wanted();
            if let Some(msg) = self.view.sync_fly_intent(want_fly) {
                self.debug.shell.push_log(msg.to_string());
                if !want_fly {
                    self.fly_input.clear_keys();
                }
            }

            if self.view.is_flycam() && !self.debug.is_open() {
                self.view.update_flycam(dt, &mut self.fly_input);
            } else if self.debug.is_open() {
                // Don't keep moving while typing in the console.
                self.fly_input.clear_keys();
            }
        }

        self.renderer.write_view_proj(self.view.view_matrix());

        #[cfg(feature = "debug-tools")]
        let draw_grid = self.debug.draw_grid();
        #[cfg(not(feature = "debug-tools"))]
        let draw_grid = true;

        let frame = self.renderer.render_scene(draw_grid)?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        #[cfg(feature = "debug-tools")]
        {
            let ppp = self.pixels_per_point();
            let time = web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now() / 1000.0)
                .unwrap_or(0.0);
            let screen_w = width as f32 / ppp;
            let screen_h = height as f32 / ppp;
            let raw = self.debug.take_raw_input(screen_w, screen_h, time);

            // Split borrows: run shell UI, then render overlay with GPU handles.
            let full = {
                let DebugTools {
                    registry, shell, ..
                } = &mut self.debug;
                shell.run_frame(registry, raw, ppp)
            };

            if let Some(full) = full {
                let mut encoder =
                    self.renderer
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("egui-encoder"),
                        });
                self.debug.shell.render_overlay(
                    debug::OverlayGpu {
                        device: &self.renderer.device,
                        queue: &self.renderer.queue,
                        encoder: &mut encoder,
                        view: &view,
                        width,
                        height,
                        pixels_per_point: ppp,
                    },
                    full,
                );
                self.renderer
                    .queue
                    .submit(std::iter::once(encoder.finish()));
            }
        }

        // Silence unused when debug-tools off.
        #[cfg(not(feature = "debug-tools"))]
        {
            let _ = view;
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

/// Read the presented canvas and hand a PNG data URL to the host sink (`window.__debugSaveShot`).
#[cfg(feature = "debug-tools")]
fn capture_canvas_png(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let data_url = canvas
        .to_data_url_with_type("image/png")
        .map_err(|e| JsValue::from_str(&format!("canvas toDataURL failed: {e:?}")))?;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let handler = js_sys::Reflect::get(&window, &JsValue::from_str("__debugSaveShot"))?;
    if handler.is_undefined() || handler.is_null() {
        return Err(JsValue::from_str(
            "window.__debugSaveShot missing (host sink not installed)",
        ));
    }
    let func = handler
        .dyn_into::<js_sys::Function>()
        .map_err(|_| JsValue::from_str("window.__debugSaveShot is not a function"))?;
    let _ = func.call1(&JsValue::NULL, &JsValue::from_str(&data_url))?;
    Ok(())
}

/// WebGPU renderer bound to a canvas, exposed to JS.
#[wasm_bindgen]
pub struct GameClient {
    inner: Rc<RefCell<ClientInner>>,
}

#[wasm_bindgen]
impl GameClient {
    /// Create a client on `canvas` and initialize WebGPU.
    #[wasm_bindgen(js_name = create)]
    pub async fn create(canvas: HtmlCanvasElement) -> Result<GameClient, JsValue> {
        console_error_panic_hook::set_once();

        let renderer = Renderer::new(canvas.clone()).await?;
        let view = ViewController::new();
        renderer.write_view_proj(view.view_matrix());

        #[cfg(feature = "debug-tools")]
        let debug = DebugTools::new(&renderer.device, renderer.config.format);

        let inner = Rc::new(RefCell::new(ClientInner {
            renderer,
            canvas,
            view,
            #[cfg(feature = "debug-tools")]
            debug,
            #[cfg(feature = "debug-tools")]
            fly_input: FlyInput::default(),
            #[cfg(feature = "debug-tools")]
            last_frame_secs: 0.0,
        }));

        web_sys::console::log_1(&"WebGPU initialized; empty scene ready".into());
        Ok(GameClient { inner })
    }

    /// Dev host bridge for the same command/cvar registry (`window.__DEBUG__`).
    #[cfg(feature = "debug-tools")]
    #[wasm_bindgen(js_name = debugHost)]
    pub fn debug_host(&self) -> DebugHost {
        DebugHost::new(self.inner.clone())
    }

    /// Draw one frame (clear, depth, debug grid, optional console).
    #[wasm_bindgen(js_name = renderFrame)]
    pub fn render_frame(&mut self) -> Result<(), JsValue> {
        self.inner.borrow_mut().render_frame()
    }

    /// Run the frame loop via `requestAnimationFrame`.
    #[wasm_bindgen(js_name = startRenderLoop)]
    pub fn start_render_loop(&self) -> Result<(), JsValue> {
        #[cfg(feature = "debug-tools")]
        {
            let canvas = self.inner.borrow().canvas.clone();
            install_input_handlers(self.inner.clone(), &canvas);
        }

        let client = self.inner.clone();
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
