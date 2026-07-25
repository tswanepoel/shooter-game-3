//! Hit marker present flash (044): X on the reticle when you claim a hit.
//!
//! Same world point as reticle / shots (weapon line). Present-only.

use game_sim::{CAMERA_VERTICAL_FOV_RAD, RETICLE_SIZE_PX};
use glam::{Mat4, Vec3};

/// Full opacity hold before fade (seconds).
const HOLD_S: f32 = 0.035;
/// Full → gone after hold (seconds). Barely a blink overall.
const FADE_S: f32 = 0.015;
/// Gap from reticle outer edge to X inner tip (framebuffer px).
const GAP_PX: f32 = 4.5;
/// Arm length past the gap (framebuffer px).
const ARM_PX: f32 = 10.5;
/// White half-thickness (framebuffer px).
const HALF_W_PX: f32 = 1.25;
/// Extra half-thickness for black border under the white arms.
const BORDER_EXTRA_PX: f32 = 1.1;

const SHADER: &str = r#"
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

/// Fade timer (present-only).
#[derive(Debug, Default)]
pub struct HitMarker {
    remaining: f32,
}

impl HitMarker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Local firer impact claim — restart full flash (hold, then fade).
    pub fn pulse(&mut self) {
        self.remaining = HOLD_S + FADE_S;
    }

    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
    }

    pub fn alpha(&self) -> f32 {
        if self.remaining <= 0.0 {
            0.0
        } else if self.remaining > FADE_S {
            1.0
        } else {
            (self.remaining / FADE_S).clamp(0.0, 1.0)
        }
    }
}

/// World-space X on the aim reticle point (alpha-blend, no depth write).
pub struct HitMarkerGpu {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl HitMarkerGpu {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hit-marker"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hit-marker-uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hit-marker-bgl"),
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
            label: Some("hit-marker-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hit-marker-pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hit-marker-pipeline"),
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
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // 4 arms × 2 passes × 6 verts = 48; headroom.
        const MAX_VERTS: usize = 64;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hit-marker-verts"),
            size: (MAX_VERTS * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bind_group,
            uniform,
            vertex_buffer,
            vertex_count: 0,
        }
    }

    /// `center` is the reticle world point (weapon line).
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        center: Option<Vec3>,
        camera_eye: Vec3,
        camera_forward: Vec3,
        screen_h_px: f32,
        alpha: f32,
    ) {
        let uniforms = FrameUniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniforms));

        if alpha <= 1e-4 {
            self.vertex_count = 0;
            return;
        }
        let Some(center) = center else {
            self.vertex_count = 0;
            return;
        };

        // Same px→world scale as reticle (015).
        let dist = (center - camera_eye).length().max(0.05);
        let half_fov = CAMERA_VERTICAL_FOV_RAD * 0.5;
        let world_h = 2.0 * dist * half_fov.tan();
        let px = world_h / screen_h_px.max(1.0);

        let forward = camera_forward.normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = if right.length_squared() < 1e-8 {
            Vec3::X
        } else {
            right.cross(forward).normalize_or_zero()
        };
        let right = if right.length_squared() < 1e-8 {
            up.cross(forward).normalize_or_zero()
        } else {
            right
        };

        let r_in = (RETICLE_SIZE_PX * 0.5 + GAP_PX) * px;
        let r_out = r_in + ARM_PX * px;
        let half_w = HALF_W_PX * px;
        let border_extra = BORDER_EXTRA_PX * px;

        let inv = std::f32::consts::FRAC_1_SQRT_2;
        // Open-centre X in the reticle billboard plane.
        let arms = [(inv, inv), (-inv, inv), (inv, -inv), (-inv, -inv)];

        let mut verts = Vec::with_capacity(48);
        let black = [0.0, 0.0, 0.0, alpha];
        let white = [1.0, 1.0, 1.0, alpha];

        for (ux, uy) in arms {
            let a0 = center + right * (ux * r_in) + up * (uy * r_in);
            let a1 = center + right * (ux * r_out) + up * (uy * r_out);
            thick_seg_billboard(
                &mut verts,
                a0,
                a1,
                forward,
                half_w + border_extra,
                border_extra,
                black,
            );
        }
        for (ux, uy) in arms {
            let a0 = center + right * (ux * r_in) + up * (uy * r_in);
            let a1 = center + right * (ux * r_out) + up * (uy * r_out);
            thick_seg_billboard(&mut verts, a0, a1, forward, half_w, 0.0, white);
        }

        self.vertex_count = verts.len() as u32;
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.vertex_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}

fn thick_seg_billboard(
    out: &mut Vec<Vertex>,
    p0: Vec3,
    p1: Vec3,
    forward: Vec3,
    half_w: f32,
    // Extra length past each tip (world); black uses this so ends are capped.
    end_pad: f32,
    color: [f32; 4],
) {
    let d = p1 - p0;
    let len = d.length().max(1e-6);
    let t = d / len;
    let p0 = p0 - t * end_pad;
    let p1 = p1 + t * end_pad;
    // Thickness stays in the reticle billboard plane (perp to view forward).
    let n = forward.cross(t).normalize_or_zero() * half_w;

    let c0 = p0 + n;
    let c1 = p0 - n;
    let c2 = p1 - n;
    let c3 = p1 + n;
    tri(out, c0, c1, c2, color);
    tri(out, c0, c2, c3, color);
}

fn tri(out: &mut Vec<Vertex>, a: Vec3, b: Vec3, c: Vec3, color: [f32; 4]) {
    out.push(Vertex {
        position: a.to_array(),
        color,
    });
    out.push(Vertex {
        position: b.to_array(),
        color,
    });
    out.push(Vertex {
        position: c.to_array(),
        color,
    });
}
