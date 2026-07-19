//! World-space aim reticle billboard on the look ray (feature 015, screen centre).

use game_sim::{CAMERA_VERTICAL_FOV_RAD, RETICLE_SIZE_PX};
use glam::{Mat4, Vec3};

const RETICLE_SHADER: &str = r#"
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

pub struct ReticleGpu {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl ReticleGpu {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("reticle"),
            source: wgpu::ShaderSource::Wgsl(RETICLE_SHADER.into()),
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reticle-uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("reticle-bgl"),
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
            label: Some("reticle-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("reticle-pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("reticle-pipeline"),
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

        // Disc (n*3) + ring (n*6); n=16 → 144 verts. Headroom for larger n.
        const MAX_VERTS: usize = 256;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reticle-verts"),
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

    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        center: Option<Vec3>,
        camera_eye: Vec3,
        camera_forward: Vec3,
        screen_h_px: f32,
    ) {
        let uniforms = FrameUniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniforms));

        let Some(center) = center else {
            self.vertex_count = 0;
            return;
        };

        let dist = (center - camera_eye).length().max(0.05);
        let half_fov = CAMERA_VERTICAL_FOV_RAD * 0.5;
        let world_h = 2.0 * dist * half_fov.tan();
        let radius = (RETICLE_SIZE_PX * 0.5) * (world_h / screen_h_px.max(1.0));

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

        let mut verts = Vec::new();
        let axes = (right, up);
        disc_tris(&mut verts, center, axes, radius, [1.0, 1.0, 1.0, 1.0], 16);
        ring_tris(
            &mut verts,
            center,
            axes,
            (radius * 0.72, radius),
            [0.0, 0.0, 0.0, 1.0],
            16,
        );

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

fn disc_tris(out: &mut Vec<Vertex>, c: Vec3, axes: (Vec3, Vec3), r: f32, color: [f32; 4], n: u32) {
    let (right, up) = axes;
    for i in 0..n {
        let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / n as f32) * std::f32::consts::TAU;
        let p0 = c + right * (a0.cos() * r) + up * (a0.sin() * r);
        let p1 = c + right * (a1.cos() * r) + up * (a1.sin() * r);
        out.push(Vertex {
            position: c.to_array(),
            color,
        });
        out.push(Vertex {
            position: p0.to_array(),
            color,
        });
        out.push(Vertex {
            position: p1.to_array(),
            color,
        });
    }
}

fn ring_tris(
    out: &mut Vec<Vertex>,
    c: Vec3,
    axes: (Vec3, Vec3),
    radii: (f32, f32),
    color: [f32; 4],
    n: u32,
) {
    let (right, up) = axes;
    let (r_in, r_out) = radii;
    for i in 0..n {
        let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / n as f32) * std::f32::consts::TAU;
        let i0 = c + right * (a0.cos() * r_in) + up * (a0.sin() * r_in);
        let i1 = c + right * (a1.cos() * r_in) + up * (a1.sin() * r_in);
        let o0 = c + right * (a0.cos() * r_out) + up * (a0.sin() * r_out);
        let o1 = c + right * (a1.cos() * r_out) + up * (a1.sin() * r_out);
        for p in [i0, o0, o1, i0, o1, i1] {
            out.push(Vertex {
                position: p.to_array(),
                color,
            });
        }
    }
}
