//! WebGPU surface, MSAA targets, debug grid, and scene pass.

use glam::Mat4;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::corpse_present::CorpsePresent;
use crate::emote_wheel::EmoteWheelGpu;
use crate::fire_fx::FireFx;
use crate::hit_marker::HitMarkerGpu;
#[cfg(feature = "debug-tools")]
use crate::lineup::LineupGpu;
use crate::remote_present::RemotePresent;
use crate::reticle::ReticleGpu;
use crate::self_present::SelfGpu;
use game_sim::{
    CAMERA_FAR_M, CAMERA_NEAR_M, CAMERA_VERTICAL_FOV_RAD, DEBUG_GRID_HALF_EXTENT_M,
    GRID_MAJOR_EVERY, GRID_MINOR_SPACING_M,
};
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.05,
    g: 0.06,
    b: 0.08,
    a: 1.0,
};

const MINOR_LINE_COLOR: [f32; 4] = [0.40, 0.45, 0.52, 0.28];
const MAJOR_LINE_COLOR: [f32; 4] = [0.65, 0.70, 0.78, 0.45];

pub(crate) const MSAA_SAMPLE_COUNT: u32 = 4;
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

pub(crate) struct Renderer {
    surface: wgpu::Surface<'static>,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) max_texture_dim: u32,
    msaa_color: wgpu::Texture,
    msaa_color_view: wgpu::TextureView,
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    pub(crate) reticle: ReticleGpu,
    pub(crate) hit_marker_gpu: HitMarkerGpu,
    pub(crate) emote_wheel_gpu: EmoteWheelGpu,
    pub(crate) fire_fx: FireFx,
}

impl Renderer {
    pub(crate) async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
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

        let reticle = ReticleGpu::new(&device, config.format, MSAA_SAMPLE_COUNT);
        let hit_marker_gpu = HitMarkerGpu::new(&device, config.format, MSAA_SAMPLE_COUNT);
        let emote_wheel_gpu = EmoteWheelGpu::new(&device, config.format, MSAA_SAMPLE_COUNT);
        let fire_fx = FireFx::new(&device, config.format, MSAA_SAMPLE_COUNT);

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
            reticle,
            hit_marker_gpu,
            emote_wheel_gpu,
            fire_fx,
        })
    }

    pub(crate) fn resize_if_needed(&mut self, width: u32, height: u32) {
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

    pub(crate) fn write_view_proj(&self, view: Mat4) -> Mat4 {
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_scene(
        &mut self,
        draw_grid: bool,
        map: Option<&crate::map_present::MapGpu>,
        self_body: Option<&SelfGpu>,
        remotes: Option<&RemotePresent>,
        corpses: Option<&CorpsePresent>,
        #[cfg(feature = "debug-tools")] lineup: Option<&LineupGpu>,
        draw_reticle: bool,
        draw_hit_marker: bool,
        draw_emote_wheel: bool,
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

            if let Some(map) = map {
                map.draw(&mut pass);
            }

            if let Some(body) = self_body {
                body.draw(&mut pass);
            }

            if let Some(remotes) = remotes {
                remotes.draw_all(&mut pass);
            }

            if let Some(corpses) = corpses {
                corpses.draw_all(&mut pass);
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

            if draw_reticle {
                self.reticle.draw(&mut pass);
            }

            if draw_hit_marker {
                self.hit_marker_gpu.draw(&mut pass);
            }

            self.fire_fx.draw(&mut pass);

            if draw_emote_wheel {
                self.emote_wheel_gpu.draw(&mut pass);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(frame)
    }
}

/// `(width, height, pixels_per_point)` â€” buffer size and buffer/CSS scale.
pub(crate) fn canvas_buffer_size(canvas: &HtmlCanvasElement, max_dim: u32) -> (u32, u32, f32) {
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
