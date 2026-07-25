use glam::Mat4;

use super::primitives::MeshVertex;
use super::shader::{FrameUniforms, AMBIENT_COLOR, KEY_COLOR, KEY_LIGHT_DIR, KIT_SHADER};

pub(crate) struct GpuPrimitive {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

pub struct MeshBatch {
    pub(crate) primitives: Vec<GpuPrimitive>,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) _texture: wgpu::Texture,
    pub(crate) _texture_view: wgpu::TextureView,
    pub(crate) _material_uniform: wgpu::Buffer,
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
            label: Some("kit-lit"),
            source: wgpu::ShaderSource::Wgsl(KIT_SHADER.into()),
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
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
            label: Some("kit-lit-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x2,
                    ],
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
        let dir = KEY_LIGHT_DIR.normalize_or_zero();
        let uniforms = FrameUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            light_dir: dir.to_array(),
            _pad0: 0.0,
            key_color: KEY_COLOR,
            _pad1: 0.0,
            ambient: AMBIENT_COLOR,
            _pad2: 0.0,
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
