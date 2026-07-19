//! Flat unlit kit meshes: load, hold pose, grip attach, GPU upload.

use std::io::Cursor;

use glam::{Mat4, Quat, Vec3};
use wasm_bindgen::JsValue;

use crate::pack::{self, Pack};

pub const KENNEY_CORE_PACK: &str = "kenney-core";

/// Character kit units → metres (2.7 kit → 1.8 m).
pub const CHAR_UNITS_TO_M: f32 = 1.0 / 1.5;
/// Blaster kit units → metres (1:1).
pub const BLASTER_UNITS_TO_M: f32 = 1.0;

/// `holding-right` on `arm-right` (−90° X).
const HOLDING_RIGHT_ROT: Quat = Quat::from_xyzw(
    std::f32::consts::FRAC_1_SQRT_2,
    0.0,
    0.0,
    -std::f32::consts::FRAC_1_SQRT_2,
);

/// Grip offsets in `arm-right` local after hold (kit units). See blaster kit README.
const BLASTER_GRIP_OFFSETS: [[f32; 3]; 18] = [
    [0.0, -1.14, 0.34], // a
    [0.0, -1.00, 0.30], // b
    [0.0, -1.11, 0.20], // c
    [0.0, -1.11, 0.18], // d
    [0.0, -2.34, 0.22], // e
    [0.0, -1.39, 0.19], // f
    [0.0, -1.27, 0.22], // g
    [0.0, -1.25, 0.24], // h
    [0.0, -0.93, 0.22], // i
    [0.0, -1.20, 0.15], // j
    [0.0, -1.09, 0.20], // k
    [0.0, -1.16, 0.20], // l
    [0.0, -1.18, 0.26], // m
    [0.0, -0.99, 0.22], // n
    [0.0, -1.06, 0.19], // o
    [0.0, -1.21, 0.14], // p
    [0.0, -1.28, 0.19], // q
    [0.0, -1.18, 0.10], // r
];

/// Muzzle points in arm-attachment frame. See blaster kit README.
pub const BLASTER_MUZZLE_POINTS: &[&[[f32; 3]]] = &[
    &[[0.0, -1.7, 0.42]],                             // a
    &[[0.0, -1.39, 0.32]],                            // b
    &[[0.0, -1.47, 0.23]],                            // c
    &[[0.0, -1.795, 0.265]],                          // d
    &[[0.07, -2.34, 0.26]],                           // e
    &[[0.0, -2.37, 0.26]],                            // f
    &[[0.0, -1.8, 0.34]],                             // g
    &[[0.0, -1.73, 0.28]],                            // h
    &[[0.0, -1.32, 0.26], [0.0, -1.32, 0.15]],        // i
    &[[-0.045, -1.655, 0.29], [0.045, -1.655, 0.29]], // j
    &[[0.0, -1.44, 0.18]],                            // k
    &[[-0.1, -1.58, 0.26], [0.1, -1.58, 0.26]],       // l
    &[[0.0, -1.65, 0.37]],                            // m
    &[[0.0, -1.47, 0.32]],                            // n
    &[
        [-0.05, -1.35, 0.25],
        [0.05, -1.35, 0.25],
        [-0.05, -1.35, 0.15],
        [0.05, -1.35, 0.15],
    ], // o
    &[[0.0, -1.855, 0.235], [0.0, -1.855, 0.14]],     // p
    &[[0.0, -1.82, 0.28], [0.0, -1.82, 0.06]],        // q
    &[[0.0, -1.81, 0.23]],                            // r
];

/// Primary muzzle in blaster-local units (future muzzle FX; not an aim basis — 015).
#[allow(dead_code)]
pub fn primary_muzzle_offset(letter_index: usize) -> Vec3 {
    let pts = BLASTER_MUZZLE_POINTS[letter_index];
    Vec3::from_array(pts[0])
}

const UNLIT_SHADER: &str = r#"
struct FrameUniforms {
    view_proj: mat4x4<f32>,
};

struct MaterialUniforms {
    base_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> frame: FrameUniforms;

@group(1) @binding(0)
var<uniform> material: MaterialUniforms;
@group(1) @binding(1)
var albedo: texture_2d<f32>;
@group(1) @binding(2)
var albedo_samp: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = frame.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(albedo, albedo_samp, in.uv);
    return tex * material.base_color;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniforms {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MaterialUniforms {
    base_color: [f32; 4],
}

struct GpuPrimitive {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

pub struct MeshBatch {
    primitives: Vec<GpuPrimitive>,
    bind_group: wgpu::BindGroup,
    _texture: wgpu::Texture,
    _texture_view: wgpu::TextureView,
    _material_uniform: wgpu::Buffer,
}

pub struct UnlitMeshGpu {
    pipeline: wgpu::RenderPipeline,
    frame_bind_group: wgpu::BindGroup,
    frame_uniform: wgpu::Buffer,
    batches: Vec<MeshBatch>,
}

pub struct UploadCtx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub material_bgl: &'a wgpu::BindGroupLayout,
    pub sampler: &'a wgpu::Sampler,
}

pub struct UnlitMeshLayout {
    pub pipeline: wgpu::RenderPipeline,
    pub frame_bind_group: wgpu::BindGroup,
    pub frame_uniform: wgpu::Buffer,
    pub material_bgl: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

impl UnlitMeshLayout {
    pub fn create(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kit-unlit"),
            source: wgpu::ShaderSource::Wgsl(UNLIT_SHADER.into()),
        });

        let frame_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kit-frame-uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let frame_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kit-frame-bgl"),
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

        let material_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kit-material-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kit-frame-bg"),
            layout: &frame_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_uniform.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kit-pipeline-layout"),
            bind_group_layouts: &[&frame_bgl, &material_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kit-unlit-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                // Blaster kit materials are double-sided.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kit-albedo-sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            frame_bind_group,
            frame_uniform,
            material_bgl,
            sampler,
        }
    }

    pub fn upload_ctx<'a>(
        &'a self,
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
    ) -> UploadCtx<'a> {
        UploadCtx {
            device,
            queue,
            material_bgl: &self.material_bgl,
            sampler: &self.sampler,
        }
    }

    pub fn finish(self, batches: Vec<MeshBatch>) -> UnlitMeshGpu {
        UnlitMeshGpu {
            pipeline: self.pipeline,
            frame_bind_group: self.frame_bind_group,
            frame_uniform: self.frame_uniform,
            batches,
        }
    }
}

impl UnlitMeshGpu {
    pub fn write_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        let uniforms = FrameUniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.frame_uniform, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn write_prim_verts(
        &self,
        queue: &wgpu::Queue,
        batch: usize,
        prim: usize,
        verts: &[MeshVertex],
    ) {
        let buf = &self.batches[batch].primitives[prim].vertex_buffer;
        queue.write_buffer(buf, 0, bytemuck::cast_slice(verts));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.frame_bind_group, &[]);

        for batch in &self.batches {
            pass.set_bind_group(1, &batch.bind_group, &[]);
            for prim in &batch.primitives {
                pass.set_vertex_buffer(0, prim.vertex_buffer.slice(..));
                pass.set_index_buffer(prim.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..prim.index_count, 0, 0..1);
            }
        }
    }
}

pub fn letter_index(letter: u8) -> Result<usize, String> {
    if (b'a'..=b'r').contains(&letter) {
        Ok((letter - b'a') as usize)
    } else {
        Err(format!("kit letter out of range: {}", letter as char))
    }
}

pub fn grip_offset(index: usize) -> Vec3 {
    Vec3::from_array(BLASTER_GRIP_OFFSETS[index])
}

pub fn kit_to_world(placement: Mat4, min_y_kit: f32) -> Mat4 {
    placement
        * Mat4::from_scale(Vec3::splat(CHAR_UNITS_TO_M))
        * Mat4::from_translation(Vec3::new(0.0, -min_y_kit, 0.0))
}

/// Grip from arm; `aim_pitch` positive = look up (presentation elevation). Lineup passes 0.
pub fn held_blaster_root(
    kit_to_world: Mat4,
    arm_right_kit: Mat4,
    letter_index: usize,
    aim_pitch: f32,
) -> Mat4 {
    let grip_kit = arm_right_kit.transform_point3(grip_offset(letter_index));
    kit_to_world
        * Mat4::from_translation(grip_kit)
        * Mat4::from_rotation_x(-aim_pitch)
        * Mat4::from_rotation_y(std::f32::consts::PI)
        * Mat4::from_scale(Vec3::splat(BLASTER_UNITS_TO_M / CHAR_UNITS_TO_M))
}

pub async fn load_kenney_core() -> Result<Pack, JsValue> {
    pack::load_pack(KENNEY_CORE_PACK).await
}

#[cfg(feature = "debug-tools")]
pub fn upload_held_pair(
    gpu: &UploadCtx<'_>,
    pack: &Pack,
    character: u8,
    blaster: u8,
    placement: Mat4,
    label: &str,
) -> Result<(MeshBatch, MeshBatch, Mat4, Mat4), String> {
    let ch = character as char;
    let bl = blaster as char;
    let bi = letter_index(blaster)?;

    let char_glb = pack
        .get(&format!("character-{ch}.mesh"))
        .map_err(|e| e.to_string())?;
    let char_png = pack
        .get(&format!("character-{ch}.albedo"))
        .map_err(|e| e.to_string())?;
    let blaster_glb = pack
        .get(&format!("blaster-{bl}.mesh"))
        .map_err(|e| e.to_string())?;
    let colormap = pack.get("blaster.colormap").map_err(|e| e.to_string())?;

    let (char_prims, min_y, arm_right_kit) = extract_character_hold(char_glb)?;
    let k2w = kit_to_world(placement, min_y);

    let char_batch = upload_batch(
        gpu,
        char_png,
        char_prims,
        k2w,
        &format!("{label}-character"),
    )?;
    let blaster_root = held_blaster_root(k2w, arm_right_kit, bi, 0.0);
    let blaster_prims = extract_primitives(blaster_glb)?;
    let blaster_batch = upload_batch(
        gpu,
        colormap,
        blaster_prims,
        blaster_root,
        &format!("{label}-blaster"),
    )?;

    Ok((char_batch, blaster_batch, k2w, arm_right_kit))
}

pub fn upload_batch(
    gpu: &UploadCtx<'_>,
    png: &[u8],
    mut prims: Vec<CpuPrim>,
    root: Mat4,
    label: &str,
) -> Result<MeshBatch, String> {
    let (tex_w, tex_h, rgba) = decode_rgba8(png)?;
    let mips = rgba_mip_levels(tex_w, tex_h, rgba);
    let mip_count = mips.len() as u32;
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: tex_w,
            height: tex_h,
            depth_or_array_layers: 1,
        },
        mip_level_count: mip_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (mw, mh, pixels)) in mips.into_iter().enumerate() {
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * mw),
                rows_per_image: Some(mh),
            },
            wgpu::Extent3d {
                width: mw,
                height: mh,
                depth_or_array_layers: 1,
            },
        );
    }
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let base_color = prims.first().map(|p| p.2).unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let material_uniform = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("kit-material"),
        size: std::mem::size_of::<MaterialUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue.write_buffer(
        &material_uniform,
        0,
        bytemuck::bytes_of(&MaterialUniforms { base_color }),
    );

    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("kit-material-bg"),
        layout: gpu.material_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: material_uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(gpu.sampler),
            },
        ],
    });

    let mut gpu_prims = Vec::with_capacity(prims.len());
    for (mut verts, indices, _) in prims.drain(..) {
        for v in &mut verts {
            let p = root.transform_point3(Vec3::from_array(v.position));
            v.position = p.to_array();
        }

        let vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kit-vertices"),
            size: (verts.len() * std::mem::size_of::<MeshVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vbuf, 0, bytemuck::cast_slice(&verts));

        let ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kit-indices"),
            size: (indices.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&ibuf, 0, bytemuck::cast_slice(&indices));

        gpu_prims.push(GpuPrimitive {
            vertex_buffer: vbuf,
            index_buffer: ibuf,
            index_count: indices.len() as u32,
        });
    }

    Ok(MeshBatch {
        primitives: gpu_prims,
        bind_group,
        _texture: texture,
        _texture_view: texture_view,
        _material_uniform: material_uniform,
    })
}

#[cfg(feature = "debug-tools")]
pub fn upload_solid_batch(
    gpu: &UploadCtx<'_>,
    prim: CpuPrim,
    root: Mat4,
    color: [f32; 4],
    label: &str,
) -> Result<MeshBatch, String> {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[255, 255, 255, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let material_uniform = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("kit-solid-material"),
        size: std::mem::size_of::<MaterialUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue.write_buffer(
        &material_uniform,
        0,
        bytemuck::bytes_of(&MaterialUniforms { base_color: color }),
    );

    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("kit-solid-material-bg"),
        layout: gpu.material_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: material_uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(gpu.sampler),
            },
        ],
    });

    let (mut verts, indices, _) = prim;
    for v in &mut verts {
        let p = root.transform_point3(Vec3::from_array(v.position));
        v.position = p.to_array();
    }

    let vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("kit-solid-vertices"),
        size: (verts.len() * std::mem::size_of::<MeshVertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue
        .write_buffer(&vbuf, 0, bytemuck::cast_slice(&verts));

    let ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("kit-solid-indices"),
        size: (indices.len() * std::mem::size_of::<u32>()) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue
        .write_buffer(&ibuf, 0, bytemuck::cast_slice(&indices));

    Ok(MeshBatch {
        primitives: vec![GpuPrimitive {
            vertex_buffer: vbuf,
            index_buffer: ibuf,
            index_count: indices.len() as u32,
        }],
        bind_group,
        _texture: texture,
        _texture_view: texture_view,
        _material_uniform: material_uniform,
    })
}

#[cfg(feature = "debug-tools")]
pub fn unit_sphere_prim(segments: u32, rings: u32) -> CpuPrim {
    let segments = segments.max(3);
    let rings = rings.max(2);
    let mut verts = Vec::new();
    let mut indices = Vec::new();

    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let phi = v * std::f32::consts::PI;
        let (sy, cy) = phi.sin_cos();
        for seg in 0..=segments {
            let u = seg as f32 / segments as f32;
            let theta = u * std::f32::consts::TAU;
            let (sx, cx) = theta.sin_cos();
            verts.push(MeshVertex {
                position: [sx * sy, cy, cx * sy],
                uv: [u, v],
            });
        }
    }

    let stride = segments + 1;
    for ring in 0..rings {
        for seg in 0..segments {
            let i0 = ring * stride + seg;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    (verts, indices, [1.0, 1.0, 1.0, 1.0])
}

pub type CpuPrim = (Vec<MeshVertex>, Vec<u32>, [f32; 4]);

/// One skinned-by-node character part: mesh in node-local space + bind local TRS.
#[derive(Clone)]
pub struct CharPart {
    pub name: String,
    pub parent: Option<usize>,
    pub bind_local: Mat4,
    pub local_verts: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    pub base_color: [f32; 4],
}

/// glTF animation path on a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimPath {
    Translation,
    Rotation,
    Scale,
}

/// One channel of a named clip (LINEAR keys).
#[derive(Clone)]
pub struct AnimChannel {
    pub node: String,
    pub path: AnimPath,
    pub times: Vec<f32>,
    /// Flat key values: 3 floats/key (T/S) or 4 (quat xyzw).
    pub values: Vec<f32>,
}

/// Named glTF clip for rigid node TRS (Kenney character kit).
#[derive(Clone)]
pub struct AnimClip {
    pub duration: f32,
    pub channels: Vec<AnimChannel>,
}

/// Sampled walk overrides for one node (only fields present in the clip).
#[derive(Clone, Copy, Default)]
struct NodeAnimSample {
    translation: Option<Vec3>,
    rotation: Option<Quat>,
    scale: Option<Vec3>,
}

impl AnimClip {
    /// Sample sparse local TRS overrides at phase ∈ [0, 1).
    fn sample_overrides(&self, phase: f32) -> std::collections::HashMap<String, NodeAnimSample> {
        use std::collections::HashMap;
        let t = phase.rem_euclid(1.0) * self.duration.max(1e-8);
        let mut out: HashMap<String, NodeAnimSample> = HashMap::new();

        for ch in &self.channels {
            let entry = out.entry(ch.node.clone()).or_default();
            match ch.path {
                AnimPath::Translation => {
                    entry.translation = sample_vec3(&ch.times, &ch.values, t);
                }
                AnimPath::Rotation => {
                    entry.rotation = sample_quat(&ch.times, &ch.values, t);
                }
                AnimPath::Scale => {
                    entry.scale = sample_vec3(&ch.times, &ch.values, t);
                }
            }
        }
        out
    }
}

fn apply_anim_to_bind(bind: Mat4, sample: NodeAnimSample) -> Mat4 {
    let (bind_scale, bind_rot, bind_trans) = bind.to_scale_rotation_translation();
    // Prefer column lengths when decomposition is unstable on near-identity.
    let scale = sample.scale.unwrap_or_else(|| {
        Vec3::new(
            bind.x_axis.truncate().length(),
            bind.y_axis.truncate().length(),
            bind.z_axis.truncate().length(),
        )
    });
    let rot = sample.rotation.unwrap_or(bind_rot);
    let trans = sample.translation.unwrap_or(bind_trans);
    let _ = bind_scale;
    Mat4::from_scale_rotation_translation(scale, rot, trans)
}

fn sample_vec3(times: &[f32], values: &[f32], t: f32) -> Option<Vec3> {
    let (i0, i1, a) = key_span(times, t)?;
    let a0 = Vec3::new(values[i0 * 3], values[i0 * 3 + 1], values[i0 * 3 + 2]);
    let a1 = Vec3::new(values[i1 * 3], values[i1 * 3 + 1], values[i1 * 3 + 2]);
    Some(a0.lerp(a1, a))
}

fn sample_quat(times: &[f32], values: &[f32], t: f32) -> Option<Quat> {
    let (i0, i1, a) = key_span(times, t)?;
    let q0 = Quat::from_xyzw(
        values[i0 * 4],
        values[i0 * 4 + 1],
        values[i0 * 4 + 2],
        values[i0 * 4 + 3],
    )
    .normalize();
    let q1 = Quat::from_xyzw(
        values[i1 * 4],
        values[i1 * 4 + 1],
        values[i1 * 4 + 2],
        values[i1 * 4 + 3],
    )
    .normalize();
    Some(q0.slerp(q1, a))
}

fn key_span(times: &[f32], t: f32) -> Option<(usize, usize, f32)> {
    if times.is_empty() {
        return None;
    }
    if times.len() == 1 {
        return Some((0, 0, 0.0));
    }
    let t = t.clamp(times[0], *times.last().unwrap());
    let mut i1 = 1;
    while i1 < times.len() && times[i1] < t {
        i1 += 1;
    }
    let i0 = i1.saturating_sub(1);
    let i1 = i1.min(times.len() - 1);
    let span = times[i1] - times[i0];
    let a = if span > 1e-8 {
        (t - times[i0]) / span
    } else {
        0.0
    };
    Some((i0, i1, a.clamp(0.0, 1.0)))
}

/// Extract a named LINEAR clip from a character GLB.
pub fn extract_clip(glb: &[u8], name: &str) -> Result<AnimClip, String> {
    let gltf = gltf::Gltf::from_slice(glb).map_err(|e| format!("gltf parse: {e}"))?;
    let blob = gltf
        .blob
        .as_ref()
        .ok_or_else(|| "GLB missing BIN chunk".to_string())?;

    let anim = gltf
        .animations()
        .find(|a| a.name() == Some(name))
        .ok_or_else(|| format!("clip '{name}' not found"))?;

    let mut channels = Vec::new();
    let mut duration = 0.0_f32;

    for channel in anim.channels() {
        let target = channel.target();
        let node_name = target.node().name().unwrap_or("").to_string();
        let path = match target.property() {
            gltf::animation::Property::Translation => AnimPath::Translation,
            gltf::animation::Property::Rotation => AnimPath::Rotation,
            gltf::animation::Property::Scale => AnimPath::Scale,
            gltf::animation::Property::MorphTargetWeights => continue,
        };

        let reader = channel.reader(|buffer| {
            if buffer.index() == 0 {
                Some(blob.as_slice())
            } else {
                None
            }
        });

        let times: Vec<f32> = reader
            .read_inputs()
            .ok_or_else(|| format!("clip '{name}' channel missing times"))?
            .collect();
        if let Some(&last) = times.last() {
            duration = duration.max(last);
        }

        let values: Vec<f32> = match path {
            AnimPath::Translation | AnimPath::Scale => {
                let outputs = reader
                    .read_outputs()
                    .ok_or_else(|| format!("clip '{name}' channel missing outputs"))?;
                match outputs {
                    gltf::animation::util::ReadOutputs::Translations(iter) => {
                        iter.flat_map(|v| [v[0], v[1], v[2]]).collect()
                    }
                    gltf::animation::util::ReadOutputs::Scales(iter) => {
                        iter.flat_map(|v| [v[0], v[1], v[2]]).collect()
                    }
                    _ => return Err(format!("clip '{name}' unexpected output for {path:?}")),
                }
            }
            AnimPath::Rotation => {
                let outputs = reader
                    .read_outputs()
                    .ok_or_else(|| format!("clip '{name}' channel missing outputs"))?;
                match outputs {
                    gltf::animation::util::ReadOutputs::Rotations(rots) => rots
                        .into_f32()
                        .flat_map(|v| [v[0], v[1], v[2], v[3]])
                        .collect(),
                    _ => return Err(format!("clip '{name}' unexpected output for rotation")),
                }
            }
        };

        channels.push(AnimChannel {
            node: node_name,
            path,
            times,
            values,
        });
    }

    if channels.is_empty() {
        return Err(format!("clip '{name}' has no channels"));
    }
    if duration <= 0.0 {
        duration = game_sim::WALK_CLIP_DURATION_S;
    }

    Ok(AnimClip { duration, channels })
}

/// Bind-pose character hierarchy with node-local meshes (no hold applied).
pub fn extract_character_parts(glb: &[u8]) -> Result<(Vec<CharPart>, f32), String> {
    let gltf = gltf::Gltf::from_slice(glb).map_err(|e| format!("gltf parse: {e}"))?;
    let blob = gltf
        .blob
        .as_ref()
        .ok_or_else(|| "GLB missing BIN chunk".to_string())?;

    let scene = gltf.default_scene().or_else(|| gltf.scenes().next());
    let roots: Vec<_> = match scene {
        Some(s) => s.nodes().collect(),
        None => gltf.nodes().collect(),
    };
    if roots.is_empty() {
        return Err("no nodes in glTF".into());
    }

    let mut parts = Vec::new();
    let mut min_y = f32::INFINITY;
    for node in roots {
        walk_parts(&node, None, blob, &mut parts, &mut min_y)?;
    }
    if parts.is_empty() {
        return Err("no mesh primitives".into());
    }
    if !min_y.is_finite() {
        min_y = 0.0;
    }
    Ok((parts, min_y))
}

fn walk_parts(
    node: &gltf::Node<'_>,
    parent: Option<usize>,
    blob: &[u8],
    parts: &mut Vec<CharPart>,
    min_y: &mut f32,
) -> Result<(), String> {
    let bind_local = local_matrix(node, false);
    let idx = parts.len();
    let mut local_verts = Vec::new();
    let mut indices = Vec::new();
    let mut base_color = [1.0, 1.0, 1.0, 1.0];
    let mut has_mesh = false;

    if let Some(mesh) = node.mesh() {
        has_mesh = true;
        for prim in mesh.primitives() {
            let reader = prim.reader(|buffer| {
                if buffer.index() == 0 {
                    Some(blob)
                } else {
                    None
                }
            });
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .ok_or_else(|| "primitive missing POSITION".to_string())?
                .collect();
            let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
                Some(tc) => tc.into_f32().collect(),
                None => vec![[0.0, 0.0]; positions.len()],
            };
            if uvs.len() != positions.len() {
                return Err("UV count != position count".into());
            }
            let prim_idx: Vec<u32> = match reader.read_indices() {
                Some(i) => i.into_u32().collect(),
                None => (0..positions.len() as u32).collect(),
            };
            let base = local_verts.len() as u32;
            for (i, p) in positions.iter().enumerate() {
                local_verts.push(MeshVertex {
                    position: *p,
                    uv: uvs[i],
                });
            }
            for i in prim_idx {
                indices.push(base + i);
            }
            base_color = material_base_color(&prim);
        }
    }

    parts.push(CharPart {
        name: node.name().unwrap_or("").to_string(),
        parent,
        bind_local,
        local_verts,
        indices,
        base_color,
    });

    if has_mesh {
        let world = node_world_bind(parts, idx);
        for v in &parts[idx].local_verts {
            let wp = world.transform_point3(Vec3::from_array(v.position));
            *min_y = min_y.min(wp.y);
        }
    }

    for child in node.children() {
        walk_parts(&child, Some(idx), blob, parts, min_y)?;
    }
    Ok(())
}

fn node_world_bind(parts: &[CharPart], idx: usize) -> Mat4 {
    let mut chain = Vec::new();
    let mut cur = Some(idx);
    while let Some(i) = cur {
        chain.push(i);
        cur = parts[i].parent;
    }
    chain.reverse();
    let mut w = Mat4::IDENTITY;
    for i in chain {
        w *= parts[i].bind_local;
    }
    w
}

/// Pose character parts from sim drive (walk clip + aim). Returns kit-space worlds and arm-right.
pub fn pose_character_kit(
    parts: &[CharPart],
    self_state: &game_sim::SelfState,
    walk_clip: Option<&AnimClip>,
) -> (Vec<Mat4>, Mat4) {
    use game_sim::LocomotionMode;

    let walk_over = match (self_state.locomotion, walk_clip) {
        (LocomotionMode::Walk, Some(clip)) => Some(clip.sample_overrides(self_state.walk_phase)),
        _ => None,
    };

    let mut locals = Vec::with_capacity(parts.len());
    for p in parts {
        let mut local = p.bind_local;
        if let Some(ref over) = walk_over {
            if let Some(sample) = over.get(&p.name) {
                // Legs and root bob from walk; upper aim layers on after.
                let apply_walk = matches!(
                    p.name.as_str(),
                    "root" | "leg-left" | "leg-right" | "arm-left"
                );
                if apply_walk {
                    local = apply_anim_to_bind(p.bind_local, *sample);
                }
            }
        }

        match p.name.as_str() {
            "torso" => {
                local *= Mat4::from_quat(Quat::from_rotation_x(-self_state.torso_pitch));
            }
            "arm-right" => {
                // Armed hold + aim owns the right arm (walk arm swing is presentation for left).
                let (_s, _r, t) = local.to_scale_rotation_translation();
                let scale = {
                    let sx = local.x_axis.truncate().length();
                    let sy = local.y_axis.truncate().length();
                    let sz = local.z_axis.truncate().length();
                    Vec3::new(sx, sy, sz)
                };
                local = Mat4::from_scale_rotation_translation(scale, HOLDING_RIGHT_ROT, t)
                    * Mat4::from_quat(Quat::from_rotation_x(-self_state.shoulder_pitch));
            }
            "head" => {
                // Look owns head attitude (015); walk head channel is unused for local self.
                let (_s, _r, t) = local.to_scale_rotation_translation();
                let scale = {
                    let sx = local.x_axis.truncate().length();
                    let sy = local.y_axis.truncate().length();
                    let sz = local.z_axis.truncate().length();
                    Vec3::new(sx, sy, sz)
                };
                let head_rot = Quat::from_rotation_y(self_state.head_yaw)
                    * Quat::from_rotation_x(-self_state.head_pitch);
                local = Mat4::from_scale_rotation_translation(scale, head_rot, t);
            }
            _ => {}
        }
        locals.push(local);
    }

    let mut worlds = vec![Mat4::IDENTITY; parts.len()];
    for i in 0..parts.len() {
        worlds[i] = match parts[i].parent {
            Some(pi) => worlds[pi] * locals[i],
            None => locals[i],
        };
    }

    let arm = parts
        .iter()
        .position(|p| p.name == "arm-right")
        .map(|i| worlds[i])
        .unwrap_or(Mat4::IDENTITY);

    (worlds, arm)
}

/// Character with `holding-right` on `arm-right`. Returns arm-right matrix in kit space.
#[cfg(feature = "debug-tools")]
pub fn extract_character_hold(glb: &[u8]) -> Result<(Vec<CpuPrim>, f32, Mat4), String> {
    let gltf = gltf::Gltf::from_slice(glb).map_err(|e| format!("gltf parse: {e}"))?;
    let blob = gltf
        .blob
        .as_ref()
        .ok_or_else(|| "GLB missing BIN chunk".to_string())?;

    let mut out = Vec::new();
    let mut min_y = f32::INFINITY;
    let mut arm_right_world = None;

    let scene = gltf.default_scene().or_else(|| gltf.scenes().next());
    let roots: Vec<_> = match scene {
        Some(s) => s.nodes().collect(),
        None => gltf.nodes().collect(),
    };

    if roots.is_empty() {
        return Err("no nodes in glTF".into());
    }

    for node in roots {
        walk_node(
            &node,
            Mat4::IDENTITY,
            blob,
            &mut out,
            &mut min_y,
            true,
            &mut arm_right_world,
        )?;
    }

    if out.is_empty() {
        return Err("no mesh primitives".into());
    }
    if !min_y.is_finite() {
        min_y = 0.0;
    }
    let arm = arm_right_world.ok_or_else(|| "missing arm-right node".to_string())?;

    Ok((out, min_y, arm))
}

pub fn extract_primitives(glb: &[u8]) -> Result<Vec<CpuPrim>, String> {
    let gltf = gltf::Gltf::from_slice(glb).map_err(|e| format!("gltf parse: {e}"))?;
    let blob = gltf
        .blob
        .as_ref()
        .ok_or_else(|| "GLB missing BIN chunk".to_string())?;

    let mut out = Vec::new();
    let mut min_y = f32::INFINITY;
    let mut arm_unused = None;

    let scene = gltf.default_scene().or_else(|| gltf.scenes().next());
    let roots: Vec<_> = match scene {
        Some(s) => s.nodes().collect(),
        None => gltf.nodes().collect(),
    };

    if roots.is_empty() {
        return Err("no nodes in glTF".into());
    }

    for node in roots {
        walk_node(
            &node,
            Mat4::IDENTITY,
            blob,
            &mut out,
            &mut min_y,
            false,
            &mut arm_unused,
        )?;
    }

    if out.is_empty() {
        return Err("no mesh primitives".into());
    }

    Ok(out)
}

fn local_matrix(node: &gltf::Node<'_>, apply_hold: bool) -> Mat4 {
    let hold = apply_hold && node.name() == Some("arm-right");
    match node.transform() {
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => {
            let t = Vec3::from_array(translation);
            let s = Vec3::from_array(scale);
            let r = if hold {
                HOLDING_RIGHT_ROT
            } else {
                Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3])
            };
            Mat4::from_scale_rotation_translation(s, r, t)
        }
        gltf::scene::Transform::Matrix { matrix } => {
            let m = Mat4::from_cols_array_2d(&matrix);
            if !hold {
                return m;
            }
            let (_scale, _rot, trans) = m.to_scale_rotation_translation();
            let scale = {
                let sx = m.x_axis.truncate().length();
                let sy = m.y_axis.truncate().length();
                let sz = m.z_axis.truncate().length();
                Vec3::new(sx, sy, sz)
            };
            Mat4::from_scale_rotation_translation(scale, HOLDING_RIGHT_ROT, trans)
        }
    }
}

fn walk_node(
    node: &gltf::Node<'_>,
    parent: Mat4,
    blob: &[u8],
    out: &mut Vec<CpuPrim>,
    min_y: &mut f32,
    apply_hold: bool,
    arm_right_world: &mut Option<Mat4>,
) -> Result<(), String> {
    let local = local_matrix(node, apply_hold);
    let world = parent * local;

    if apply_hold && node.name() == Some("arm-right") {
        *arm_right_world = Some(world);
    }

    if let Some(mesh) = node.mesh() {
        for prim in mesh.primitives() {
            let reader = prim.reader(|buffer| {
                if buffer.index() == 0 {
                    Some(blob)
                } else {
                    None
                }
            });

            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .ok_or_else(|| "primitive missing POSITION".to_string())?
                .collect();
            let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
                Some(tc) => tc.into_f32().collect(),
                None => vec![[0.0, 0.0]; positions.len()],
            };
            if uvs.len() != positions.len() {
                return Err("UV count != position count".into());
            }

            let indices: Vec<u32> = match reader.read_indices() {
                Some(idx) => idx.into_u32().collect(),
                None => (0..positions.len() as u32).collect(),
            };

            let mut verts = Vec::with_capacity(positions.len());
            for (i, p) in positions.iter().enumerate() {
                let wp = world.transform_point3(Vec3::from_array(*p));
                *min_y = min_y.min(wp.y);
                verts.push(MeshVertex {
                    position: wp.to_array(),
                    uv: uvs[i],
                });
            }

            let base_color = material_base_color(&prim);
            out.push((verts, indices, base_color));
        }
    }

    for child in node.children() {
        walk_node(&child, world, blob, out, min_y, apply_hold, arm_right_world)?;
    }
    Ok(())
}

fn material_base_color(prim: &gltf::Primitive<'_>) -> [f32; 4] {
    prim.material().pbr_metallic_roughness().base_color_factor()
}

fn rgba_mip_levels(width: u32, height: u32, rgba: Vec<u8>) -> Vec<(u32, u32, Vec<u8>)> {
    let mut levels = Vec::new();
    let mut w = width.max(1);
    let mut h = height.max(1);
    let mut data = rgba;
    loop {
        levels.push((w, h, data.clone()));
        if w == 1 && h == 1 {
            break;
        }
        let nw = (w / 2).max(1);
        let nh = (h / 2).max(1);
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                let x0 = x * 2;
                let y0 = y * 2;
                let x1 = (x0 + 1).min(w - 1);
                let y1 = (y0 + 1).min(h - 1);
                let mut acc = [0u32; 4];
                for (sx, sy) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                    let i = ((sy * w + sx) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += data[i + c] as u32;
                    }
                }
                let o = ((y * nw + x) * 4) as usize;
                for c in 0..4 {
                    next[o + c] = (acc[c] / 4) as u8;
                }
            }
        }
        w = nw;
        h = nh;
        data = next;
    }
    levels
}

fn decode_rgba8(png_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("png header: {e}"))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png frame: {e}"))?;
    let w = info.width;
    let h = info.height;
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => {
            let rgb = &buf[..info.buffer_size()];
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for chunk in rgb.chunks_exact(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            rgba
        }
        png::ColorType::Grayscale => {
            let g = &buf[..info.buffer_size()];
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for &v in g {
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
            rgba
        }
        png::ColorType::GrayscaleAlpha => {
            let ga = &buf[..info.buffer_size()];
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for chunk in ga.chunks_exact(2) {
                rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
            rgba
        }
        other => return Err(format!("unsupported png color type: {other:?}")),
    };
    Ok((w, h, rgba))
}
