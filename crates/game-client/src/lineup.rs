//! Debug blaster lineup (feature 011): each blaster held by a Kenney character.
//! Kit facts: `assets/source/characters/README.md`, `assets/source/blasters/README.md`.
//! Loads via cook pack `kenney-core` (feature 010), not source paths.

use std::io::Cursor;

use glam::{Mat4, Quat, Vec3};
use wasm_bindgen::JsValue;

use crate::pack;

const LETTERS: &[u8] = b"abcdefghijklmnopqr";
/// Pack id from cook manifest (demand-cadence core art).
const KENNEY_CORE_PACK: &str = "kenney-core";

// --- Kit → world metres (independent truths; see kit READMEs) ---
/// Character authored units → metres (2.7 kit → 1.8 m standing).
const CHAR_UNITS_TO_M: f32 = 1.0 / 1.5;
/// Blaster authored units → metres (1:1; do not share character scale).
const BLASTER_UNITS_TO_M: f32 = 1.0;

/// Row spacing (m); wider than character-only so held weapons clear neighbours.
const LINEUP_SPACING_M: f32 = 2.0;
/// Row depth (m); stub cam looks −Z.
const LINEUP_Z_M: f32 = -6.0;

/// `holding-right` on `arm-right` only (character draw). −90° X: hand axis → character +Z.
const HOLDING_RIGHT_ROT: Quat = Quat::from_xyzw(
    std::f32::consts::FRAC_1_SQRT_2,
    0.0,
    0.0,
    -std::f32::consts::FRAC_1_SQRT_2,
);

/// Per-blaster grip offset in `arm-right` local after hold (character kit units).
/// Source: `assets/source/blasters/README.md` — keep in sync.
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
struct MeshVertex {
    position: [f32; 3],
    uv: [f32; 2],
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

/// One textured mesh batch (character or blaster).
struct MeshBatch {
    primitives: Vec<GpuPrimitive>,
    bind_group: wgpu::BindGroup,
    _texture: wgpu::Texture,
    _texture_view: wgpu::TextureView,
    _material_uniform: wgpu::Buffer,
}

pub struct LineupGpu {
    pipeline: wgpu::RenderPipeline,
    frame_bind_group: wgpu::BindGroup,
    frame_uniform: wgpu::Buffer,
    batches: Vec<MeshBatch>,
}

impl LineupGpu {
    pub async fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Result<Self, JsValue> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lineup-unlit"),
            source: wgpu::ShaderSource::Wgsl(UNLIT_SHADER.into()),
        });

        let frame_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lineup-frame-uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let frame_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lineup-frame-bgl"),
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
            label: Some("lineup-material-bgl"),
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
            label: Some("lineup-frame-bg"),
            layout: &frame_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_uniform.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lineup-pipeline-layout"),
            bind_group_layouts: &[&frame_bgl, &material_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lineup-unlit-pipeline"),
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
            label: Some("lineup-albedo-sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pack = pack::load_pack(KENNEY_CORE_PACK).await?;
        let colormap = pack
            .get("blaster.colormap")
            .map_err(|e| JsValue::from_str(&e))?;

        let n = LETTERS.len();
        let mut batches = Vec::with_capacity(n * 2);

        for (i, &letter) in LETTERS.iter().enumerate() {
            let ch = letter as char;
            let mesh_id = format!("character-{ch}.mesh");
            let albedo_id = format!("character-{ch}.albedo");
            let blaster_id = format!("blaster-{ch}.mesh");
            let char_glb = pack.get(&mesh_id).map_err(|e| JsValue::from_str(&e))?;
            let char_png = pack.get(&albedo_id).map_err(|e| JsValue::from_str(&e))?;
            let blaster_glb = pack.get(&blaster_id).map_err(|e| JsValue::from_str(&e))?;

            let x = (i as f32 - (n as f32 - 1.0) * 0.5) * LINEUP_SPACING_M;
            let placement = Mat4::from_translation(Vec3::new(x, 0.0, LINEUP_Z_M));

            let (char_prims, min_y, arm_right_kit) = extract_character_hold(char_glb)
                .map_err(|e| JsValue::from_str(&format!("character-{ch}: {e}")))?;
            // Shared path into world metres for both kits (feet on y = 0, row placement).
            let kit_to_world = placement
                * Mat4::from_scale(Vec3::splat(CHAR_UNITS_TO_M))
                * Mat4::from_translation(Vec3::new(0.0, -min_y, 0.0));

            let gpu = UploadCtx {
                device,
                queue,
                material_bgl: &material_bgl,
                sampler: &sampler,
            };
            let char_batch =
                upload_batch(&gpu, char_png, char_prims, kit_to_world, "lineup-character")
                    .map_err(|e| JsValue::from_str(&format!("character-{ch} upload: {e}")))?;
            batches.push(char_batch);

            // Arm hierarchy → grip *position* only. Blaster pose is character-frame:
            // Ry(π) maps mesh −Z muzzle → character +Z; scale is BLASTER_UNITS_TO_M.
            let grip_kit =
                arm_right_kit.transform_point3(Vec3::from_array(BLASTER_GRIP_OFFSETS[i]));
            let blaster_root = kit_to_world
                * Mat4::from_translation(grip_kit)
                * Mat4::from_rotation_y(std::f32::consts::PI)
                * Mat4::from_scale(Vec3::splat(BLASTER_UNITS_TO_M / CHAR_UNITS_TO_M));
            let blaster_prims = extract_primitives(blaster_glb)
                .map_err(|e| JsValue::from_str(&format!("blaster-{ch}: {e}")))?;
            let blaster_batch = upload_batch(
                &gpu,
                colormap,
                blaster_prims,
                blaster_root,
                "lineup-blaster",
            )
            .map_err(|e| JsValue::from_str(&format!("blaster-{ch} upload: {e}")))?;
            batches.push(blaster_batch);
        }

        Ok(Self {
            pipeline,
            frame_bind_group,
            frame_uniform,
            batches,
        })
    }

    pub fn write_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        let uniforms = FrameUniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.frame_uniform, 0, bytemuck::bytes_of(&uniforms));
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

struct UploadCtx<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    material_bgl: &'a wgpu::BindGroupLayout,
    sampler: &'a wgpu::Sampler,
}

fn upload_batch(
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
        label: Some("lineup-material"),
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
        label: Some("lineup-material-bg"),
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
            label: Some("lineup-vertices"),
            size: (verts.len() * std::mem::size_of::<MeshVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vbuf, 0, bytemuck::cast_slice(&verts));

        let ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lineup-indices"),
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

type CpuPrim = (Vec<MeshVertex>, Vec<u32>, [f32; 4]);

/// Character hierarchy with `holding-right` on `arm-right`. Returns arm-right world (kit space).
fn extract_character_hold(glb: &[u8]) -> Result<(Vec<CpuPrim>, f32, Mat4), String> {
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

fn extract_primitives(glb: &[u8]) -> Result<Vec<CpuPrim>, String> {
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
            // Decompose so we can replace rotation with hold pose.
            let (_scale, _rot, trans) = m.to_scale_rotation_translation();
            let scale = {
                // Preserve authored scale from matrix columns lengths.
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

/// Albedo mip chain (box filter).
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

#[derive(Default)]
pub enum LineupState {
    #[default]
    Idle,
    Loading,
    Ready(LineupGpu),
    #[allow(dead_code)]
    Failed(String),
}
