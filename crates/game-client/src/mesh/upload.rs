use std::io::Cursor;

use glam::Mat4;

#[cfg(feature = "debug-tools")]
use crate::pack::Pack;

#[cfg(feature = "debug-tools")]
use super::gltf::{extract_character_hold, extract_primitives};
use super::gpu::{GpuPrimitive, MeshBatch, UploadCtx};
#[cfg(feature = "debug-tools")]
use super::kit::{held_blaster_root, kit_to_world, letter_index};
use super::primitives::{transform_vertex, CpuPrim, MeshVertex};
use super::shader::MaterialUniforms;

/// Upload character + held blaster. Returns batches and the **held_blaster** root (037)
/// so callers (lineup muzzles) sample blaster-local points under the same matrix.
#[cfg(feature = "debug-tools")]
pub fn upload_held_pair(
    gpu: &UploadCtx<'_>,
    pack: &Pack,
    character: u8,
    blaster: u8,
    placement: Mat4,
    label: &str,
) -> Result<(MeshBatch, MeshBatch, Mat4), String> {
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
    let blaster_root = held_blaster_root(k2w, arm_right_kit, bi);
    let blaster_prims = extract_primitives(blaster_glb)?;
    let blaster_batch = upload_batch(
        gpu,
        colormap,
        blaster_prims,
        blaster_root,
        &format!("{label}-blaster"),
    )?;

    Ok((char_batch, blaster_batch, blaster_root))
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
        bytemuck::bytes_of(&MaterialUniforms {
            base_color,
            flags: [1.0, 0.0, 0.0, 0.0],
        }),
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
            transform_vertex(v, root);
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
        bytemuck::bytes_of(&MaterialUniforms {
            base_color: color,
            flags: [0.0, 0.0, 0.0, 0.0],
        }),
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
        transform_vertex(v, root);
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
