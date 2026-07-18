//! WebGPU client: mounted self view, input session, optional debug tools.

#![cfg(target_arch = "wasm32")]

#[cfg(feature = "debug-tools")]
mod debug;
mod input;
#[cfg(feature = "debug-tools")]
mod lineup;
mod mesh_unlit;
mod pack;
mod self_present;
mod view;

use std::cell::RefCell;
use std::rc::Rc;

use game_sim::{
    SelfState, CAMERA_FAR_M, CAMERA_NEAR_M, CAMERA_VERTICAL_FOV_RAD, DEBUG_GRID_HALF_EXTENT_M,
    GRID_MAJOR_EVERY, GRID_MINOR_SPACING_M,
};
use glam::Mat4;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[cfg(feature = "debug-tools")]
use debug::{DebugHost, DebugTools};
use input::{install_input_handlers, InputSession};
#[cfg(feature = "debug-tools")]
use lineup::{LineupGpu, LineupState};
use self_present::{SelfGpu, SelfPresentState};
#[cfg(feature = "debug-tools")]
use view::FlyInput;
use view::ViewController;

const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.05,
    g: 0.06,
    b: 0.08,
    a: 1.0,
};

const MINOR_LINE_COLOR: [f32; 4] = [0.40, 0.45, 0.52, 0.28];
const MAJOR_LINE_COLOR: [f32; 4] = [0.65, 0.70, 0.78, 0.45];

const MSAA_SAMPLE_COUNT: u32 = 4;
const MAX_DEVICE_PIXEL_RATIO: f32 = 3.0;

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
    max_texture_dim: u32,
    msaa_color: wgpu::Texture,
    msaa_color_view: wgpu::TextureView,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl Renderer {
    async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {e}")))?;

        // SAFETY: canvas handle is copied; surface lives for the page.
        let surface: wgpu::Surface<'static> = unsafe { std::mem::transmute(surface) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| JsValue::from_str("No WebGPU adapter available"))?;

        let max_texture_dim = adapter.limits().max_texture_dimension_2d;
        let (width, height, _ppp) = canvas_buffer_size(&canvas, max_texture_dim);
        canvas.set_width(width);
        canvas.set_height(height);

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
        // Display-referred Unorm for the unlit atlas path.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
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

        let (msaa_color, msaa_color_view) = create_msaa_color(&device, format, width, height);
        let (depth, depth_view) = create_depth(&device, width, height);

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
                // Transparent grid: depth test only.
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
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
            max_texture_dim,
            msaa_color,
            msaa_color_view,
            depth,
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
        let (msaa_color, msaa_color_view) =
            create_msaa_color(&self.device, self.config.format, width, height);
        let (depth, depth_view) = create_depth(&self.device, width, height);
        self.msaa_color = msaa_color;
        self.msaa_color_view = msaa_color_view;
        self.depth = depth;
        self.depth_view = depth_view;
    }

    fn write_view_proj(&self, view: Mat4) -> Mat4 {
        let aspect = self.config.width as f32 / self.config.height as f32;
        let proj =
            perspective_rh_wgpu(CAMERA_VERTICAL_FOV_RAD, aspect, CAMERA_NEAR_M, CAMERA_FAR_M);
        let view_proj = proj * view;
        let uniforms = FrameUniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
        view_proj
    }

    fn render_scene(
        &mut self,
        draw_grid: bool,
        self_body: Option<&SelfGpu>,
        #[cfg(feature = "debug-tools")] lineup: Option<&LineupGpu>,
    ) -> Result<wgpu::SurfaceTexture, JsValue> {
        let frame = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to acquire swapchain texture: {e}")))?;
        let resolve_view = frame
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
                    view: &self.msaa_color_view,
                    resolve_target: Some(&resolve_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if let Some(body) = self_body {
                body.draw(&mut pass);
            }

            #[cfg(feature = "debug-tools")]
            if let Some(lineup) = lineup {
                lineup.draw(&mut pass);
            }

            if draw_grid {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.draw(0..self.vertex_count, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        // Caller may paint UI on the resolved swapchain, then present.
        Ok(frame)
    }
}

/// `(width, height, pixels_per_point)` — buffer size and buffer/CSS scale.
fn canvas_buffer_size(canvas: &HtmlCanvasElement, max_dim: u32) -> (u32, u32, f32) {
    let css_w = canvas.client_width().max(1) as f32;
    let css_h = canvas.client_height().max(1) as f32;
    let dpr = web_sys::window()
        .map(|w| w.device_pixel_ratio() as f32)
        .unwrap_or(1.0)
        .clamp(1.0, MAX_DEVICE_PIXEL_RATIO);
    let mut width = (css_w * dpr).round().max(1.0) as u32;
    let mut height = (css_h * dpr).round().max(1.0) as u32;
    let max_dim = max_dim.max(1);
    if width > max_dim || height > max_dim {
        let scale = (max_dim as f32 / width as f32).min(max_dim as f32 / height as f32);
        width = ((width as f32) * scale).floor().max(1.0) as u32;
        height = ((height as f32) * scale).floor().max(1.0) as u32;
    }
    let ppp = width as f32 / css_w;
    (width, height, ppp)
}

fn create_msaa_color(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa-color"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: MSAA_SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_depth(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: MSAA_SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    (depth, view)
}

/// Right-handed perspective, clip depth 0..1 (WebGPU).
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

/// World-space line list on y = 0.
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

pub(crate) struct ClientInner {
    renderer: Renderer,
    canvas: HtmlCanvasElement,
    /// Buffer / CSS size (HiDPI).
    pixels_per_point: f32,
    pub(crate) self_state: SelfState,
    pub(crate) view: ViewController,
    pub(crate) session: InputSession,
    self_present: SelfPresentState,
    #[cfg(feature = "debug-tools")]
    pub(crate) debug: DebugTools,
    #[cfg(feature = "debug-tools")]
    pub(crate) fly_input: FlyInput,
    #[cfg(feature = "debug-tools")]
    last_frame_secs: f64,
    #[cfg(feature = "debug-tools")]
    lineup: LineupState,
}

impl ClientInner {
    #[cfg(feature = "debug-tools")]
    pub(crate) fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    fn render_frame(&mut self) -> Result<(), JsValue> {
        let (width, height, ppp) = canvas_buffer_size(&self.canvas, self.renderer.max_texture_dim);
        self.pixels_per_point = ppp;
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
            if let Some(msg) = self.view.sync_fly_intent(want_fly, &self.self_state) {
                self.debug.shell.push_log(msg.to_string());
                if !want_fly {
                    self.fly_input.clear_keys();
                }
            }

            let look = self.session.take_look_px();
            let session_ok = self.session.is_active();
            let console_open = self.debug.is_open();

            if session_ok && self.view.is_flycam() && !console_open {
                self.view.update_flycam(dt, &self.fly_input, look);
            } else if console_open || !session_ok {
                self.fly_input.clear_keys();
            }
        }

        #[cfg(not(feature = "debug-tools"))]
        {
            let _ = self.session.take_look_px();
        }

        let view_proj = self
            .renderer
            .write_view_proj(self.view.view_matrix(&self.self_state));

        if let SelfPresentState::Ready(gpu) = &self.self_present {
            gpu.write_view_proj(&self.renderer.queue, view_proj);
        }
        let self_ref = match &self.self_present {
            SelfPresentState::Ready(gpu) => Some(gpu),
            _ => None,
        };

        #[cfg(feature = "debug-tools")]
        let draw_grid = self.debug.draw_grid();
        #[cfg(not(feature = "debug-tools"))]
        let draw_grid = true;

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
            self.renderer
                .render_scene(draw_grid, self_ref, lineup_ref)?
        };
        #[cfg(not(feature = "debug-tools"))]
        let frame = self.renderer.render_scene(draw_grid, self_ref)?;
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

fn maybe_kick_self_load(inner: &Rc<RefCell<ClientInner>>) {
    let should_start = {
        let c = inner.borrow();
        matches!(c.self_present, SelfPresentState::Idle)
    };
    if !should_start {
        return;
    }

    {
        let mut c = inner.borrow_mut();
        c.self_present = SelfPresentState::Loading;
    }

    let (device, queue, format, self_state) = {
        let c = inner.borrow();
        (
            c.renderer.device.clone(),
            c.renderer.queue.clone(),
            c.renderer.config.format,
            c.self_state.clone(),
        )
    };

    let inner = inner.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = SelfGpu::load(&device, &queue, format, MSAA_SAMPLE_COUNT, &self_state).await;
        let mut c = inner.borrow_mut();
        match result {
            Ok(gpu) => {
                web_sys::console::log_1(&"self: body and blaster ready".into());
                c.self_present = SelfPresentState::Ready(gpu);
            }
            Err(err) => {
                let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                web_sys::console::error_1(&JsValue::from_str(&format!("self load failed: {msg}")));
                c.self_present = SelfPresentState::Failed(msg);
            }
        }
    });
}

#[cfg(feature = "debug-tools")]
fn maybe_kick_lineup_load(inner: &Rc<RefCell<ClientInner>>) {
    {
        let mut c = inner.borrow_mut();
        if !c.debug.draw_lineup() {
            if matches!(c.lineup, LineupState::Failed(_)) {
                c.lineup = LineupState::Idle;
            }
            return;
        }
    }

    let should_start = {
        let c = inner.borrow();
        matches!(c.lineup, LineupState::Idle)
    };
    if !should_start {
        return;
    }

    {
        let mut c = inner.borrow_mut();
        c.lineup = LineupState::Loading;
        c.debug.shell.push_log("lineup: loading blaster lineup…");
    }

    let (device, queue, format) = {
        let c = inner.borrow();
        (
            c.renderer.device.clone(),
            c.renderer.queue.clone(),
            c.renderer.config.format,
        )
    };

    let inner = inner.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = LineupGpu::load(&device, &queue, format, MSAA_SAMPLE_COUNT).await;
        let mut c = inner.borrow_mut();
        match result {
            Ok(gpu) => {
                c.debug.shell.push_log("lineup: ready");
                c.lineup = LineupState::Ready(gpu);
            }
            Err(err) => {
                let msg = err.as_string().unwrap_or_else(|| format!("{err:?}"));
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "lineup load failed: {msg}"
                )));
                c.debug.shell.push_log(format!("lineup failed: {msg}"));
                c.lineup = LineupState::Failed(msg);
            }
        }
    });
}

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

#[wasm_bindgen]
pub struct GameClient {
    inner: Rc<RefCell<ClientInner>>,
}

#[wasm_bindgen]
impl GameClient {
    #[wasm_bindgen(js_name = create)]
    pub async fn create(canvas: HtmlCanvasElement) -> Result<GameClient, JsValue> {
        console_error_panic_hook::set_once();

        let renderer = Renderer::new(canvas.clone()).await?;
        let self_state = SelfState::default_loadout();
        let view = ViewController::new();
        renderer.write_view_proj(view.view_matrix(&self_state));

        #[cfg(feature = "debug-tools")]
        let debug = DebugTools::new(&renderer.device, renderer.config.format);

        let ppp = canvas_buffer_size(&canvas, renderer.max_texture_dim).2;
        let inner = Rc::new(RefCell::new(ClientInner {
            renderer,
            canvas,
            pixels_per_point: ppp,
            self_state,
            view,
            session: InputSession::new(),
            self_present: SelfPresentState::Idle,
            #[cfg(feature = "debug-tools")]
            debug,
            #[cfg(feature = "debug-tools")]
            fly_input: FlyInput::default(),
            #[cfg(feature = "debug-tools")]
            last_frame_secs: 0.0,
            #[cfg(feature = "debug-tools")]
            lineup: LineupState::Idle,
        }));

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
