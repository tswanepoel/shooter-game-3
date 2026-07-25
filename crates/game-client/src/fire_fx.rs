//! Muzzle flash and optional projectile tracers.

use std::collections::HashMap;

use game_net::PlayerId;
use game_sim::{weapon_def, Projectile, SelfState, WeaponDef};
use glam::{Mat4, Vec3};

/// Flash sphere radius (m).
const FLASH_RADIUS_M: f32 = 0.03;
/// Flash lifetime (s).
const FLASH_LIFE_S: f32 = 0.05;
/// Warm muzzle flash colour (unlit).
const FLASH_COLOR: [f32; 4] = [1.0, 0.72, 0.28, 0.95];
/// Debug tracer colour.
const TRACER_COLOR: [f32; 4] = [1.0, 0.85, 0.2, 0.85];
const TRACER_HALF_LEN_M: f32 = 0.35;
const TRACER_HALF_W_M: f32 = 0.008;

const MAX_FLASH_VERTS: usize = 2048;
const MAX_TRACER_VERTS: usize = 4096;

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

const FX_SHADER: &str = r#"
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

#[derive(Clone, Copy)]
struct Flash {
    /// `None` = local self; `Some(id)` = remote peer.
    owner: Option<PlayerId>,
    /// Kit muzzle index (037 table); rebound each frame to present pose.
    muzzle_index: u8,
    age: f32,
    /// Live world centre (seeded at discharge, overwritten by rebind).
    pos: Vec3,
}

/// Muzzle flash, peer fire residual, optional tracers.
pub struct FireFx {
    flashes: Vec<Flash>,
    remote_residual: HashMap<PlayerId, SelfState>,
    pub show_tracers: bool,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    flash_vbuf: wgpu::Buffer,
    flash_count: u32,
    tracer_vbuf: wgpu::Buffer,
    tracer_count: u32,
}

impl FireFx {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fire-fx"),
            source: wgpu::ShaderSource::Wgsl(FX_SHADER.into()),
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fire-fx-uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fire-fx-bgl"),
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
            label: Some("fire-fx-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fire-fx-pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fire-fx-pipeline"),
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
                depth_compare: wgpu::CompareFunction::LessEqual,
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

        let flash_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fire-fx-flash-verts"),
            size: (MAX_FLASH_VERTS * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let tracer_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fire-fx-tracer-verts"),
            size: (MAX_TRACER_VERTS * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            flashes: Vec::new(),
            remote_residual: HashMap::new(),
            show_tracers: false,
            pipeline,
            bind_group,
            uniform,
            flash_vbuf,
            flash_count: 0,
            tracer_vbuf,
            tracer_count: 0,
        }
    }

    pub fn note_self_discharge(&mut self, muzzle_indices: &[u8], seed_worlds: &[Vec3]) {
        for (i, &mi) in muzzle_indices.iter().enumerate() {
            let pos = seed_worlds.get(i).copied().unwrap_or(Vec3::ZERO);
            self.flashes.push(Flash {
                owner: None,
                muzzle_index: mi,
                age: 0.0,
                pos,
            });
        }
    }

    pub fn note_remote_discharge(
        &mut self,
        id: PlayerId,
        def: &WeaponDef,
        muzzle_indices: &[u8],
        seed_worlds: &[Vec3],
        yaw_sign: f32,
    ) {
        for (i, &mi) in muzzle_indices.iter().enumerate() {
            let pos = seed_worlds.get(i).copied().unwrap_or(Vec3::ZERO);
            self.flashes.push(Flash {
                owner: Some(id),
                muzzle_index: mi,
                age: 0.0,
                pos,
            });
        }
        let entry = self
            .remote_residual
            .entry(id)
            .or_insert_with(SelfState::default_loadout);
        entry.apply_fire_impulse(def, yaw_sign);
    }

    pub fn note_peer_projectiles(
        &mut self,
        id: PlayerId,
        weapon: u8,
        muzzle_indices: &[u8],
        seed_worlds: &[Vec3],
    ) {
        let Some(def) = weapon_def(weapon) else {
            return;
        };
        let yaw = if id.wrapping_mul(2654435761) & 1 == 0 {
            1.0
        } else {
            -1.0
        };
        self.note_remote_discharge(id, def, muzzle_indices, seed_worlds, yaw);
    }

    pub fn apply_remote_fire_residual(&self, id: PlayerId, state: &mut SelfState) {
        if let Some(src) = self.remote_residual.get(&id) {
            src.copy_fire_residual_to(state);
        }
    }

    pub fn rebind_positions(
        &mut self,
        mut resolve: impl FnMut(Option<PlayerId>, u8) -> Option<Vec3>,
    ) {
        for f in &mut self.flashes {
            if let Some(p) = resolve(f.owner, f.muzzle_index) {
                f.pos = p;
            }
        }
    }

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        for f in &mut self.flashes {
            f.age += dt;
        }
        self.flashes.retain(|f| f.age < FLASH_LIFE_S);
        for s in self.remote_residual.values_mut() {
            s.tick_joint_residual(dt, false);
        }
        self.remote_residual.retain(|_, s| s.has_fire_residual());
    }

    pub fn update_draw(
        &mut self,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        _camera_eye: Vec3,
        camera_forward: Vec3,
        projectiles: &[Projectile],
    ) {
        let uniforms = FrameUniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniforms));

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
        let axes = (right, up);

        let mut flash_verts = Vec::new();
        for f in &self.flashes {
            let t = (f.age / FLASH_LIFE_S).clamp(0.0, 1.0);
            let fade = 1.0 - t;
            let r = FLASH_RADIUS_M * (0.85 + 0.3 * (1.0 - t));
            let mut c = FLASH_COLOR;
            c[3] *= fade;
            disc_tris(&mut flash_verts, f.pos, axes, r, c, 12);
            if flash_verts.len() >= MAX_FLASH_VERTS {
                break;
            }
        }
        self.flash_count = flash_verts.len().min(MAX_FLASH_VERTS) as u32;
        if !flash_verts.is_empty() {
            queue.write_buffer(
                &self.flash_vbuf,
                0,
                bytemuck::cast_slice(&flash_verts[..self.flash_count as usize]),
            );
        }

        let mut tracer_verts = Vec::new();
        if self.show_tracers {
            for p in projectiles {
                let dir = p.velocity.normalize_or_zero();
                if dir.length_squared() < 1e-12 {
                    continue;
                }
                let side = dir.cross(Vec3::Y).normalize_or_zero();
                let side = if side.length_squared() < 1e-12 {
                    dir.cross(Vec3::X).normalize_or_zero()
                } else {
                    side
                };
                let a = p.position - dir * TRACER_HALF_LEN_M;
                let b = p.position + dir * TRACER_HALF_LEN_M;
                let hw = side * TRACER_HALF_W_M;
                // two tris quad
                let c0 = a - hw;
                let c1 = a + hw;
                let c2 = b + hw;
                let c3 = b - hw;
                push_tri(&mut tracer_verts, c0, c1, c2, TRACER_COLOR);
                push_tri(&mut tracer_verts, c0, c2, c3, TRACER_COLOR);
                if tracer_verts.len() >= MAX_TRACER_VERTS {
                    break;
                }
            }
        }
        self.tracer_count = tracer_verts.len().min(MAX_TRACER_VERTS) as u32;
        if !tracer_verts.is_empty() {
            queue.write_buffer(
                &self.tracer_vbuf,
                0,
                bytemuck::cast_slice(&tracer_verts[..self.tracer_count as usize]),
            );
        }
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.flash_count == 0 && self.tracer_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        if self.flash_count > 0 {
            pass.set_vertex_buffer(0, self.flash_vbuf.slice(..));
            pass.draw(0..self.flash_count, 0..1);
        }
        if self.tracer_count > 0 {
            pass.set_vertex_buffer(0, self.tracer_vbuf.slice(..));
            pass.draw(0..self.tracer_count, 0..1);
        }
    }
}

fn push_tri(out: &mut Vec<Vertex>, a: Vec3, b: Vec3, c: Vec3, color: [f32; 4]) {
    for p in [a, b, c] {
        out.push(Vertex {
            position: p.to_array(),
            color,
        });
    }
}

fn disc_tris(
    out: &mut Vec<Vertex>,
    center: Vec3,
    axes: (Vec3, Vec3),
    radius: f32,
    color: [f32; 4],
    n: u32,
) {
    let (right, up) = axes;
    let n = n.max(3);
    for i in 0..n {
        let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / n as f32) * std::f32::consts::TAU;
        let p0 = center;
        let p1 = center + right * (a0.cos() * radius) + up * (a0.sin() * radius);
        let p2 = center + right * (a1.cos() * radius) + up * (a1.sin() * radius);
        push_tri(out, p0, p1, p2, color);
    }
}
