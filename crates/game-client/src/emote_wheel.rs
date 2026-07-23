//! Emote wheel state + GPU radial overlay (039).
//!
//! Hold B; mouse select; release commits or centre dead-zone cancels.
//! Drawn in clip-space (NDC) after the scene — no DOM/CSS.

use game_sim::{emote_slot_from_select, EMOTE_CATALOG, EMOTE_WHEEL_DEADZONE};

/// Mouse-select accumulate scale: ~120 px from centre to full radius.
const SELECT_PX_SCALE: f32 = 1.0 / 120.0;

/// Outer radius in NDC (half-height units).
const R_OUTER: f32 = 0.42;
/// Inner hole (dead-zone visual).
const R_INNER: f32 = 0.11;
/// Label ring (mid of annulus).
const R_LABEL: f32 = 0.28;
/// Segments per wedge arc.
const ARC_SEGS: u32 = 12;

const SHADER: &str = r#"
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
    // Already NDC (xy); ignore z.
    out.clip_position = vec4<f32>(in.position.xy, 0.0, 1.0);
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

/// Input / selection state (no GPU).
#[derive(Debug, Default)]
pub struct EmoteWheel {
    open: bool,
    select_x: f32,
    select_y: f32,
}

impl EmoteWheel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.open = true;
        self.select_x = 0.0;
        self.select_y = 0.0;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.select_x = 0.0;
        self.select_y = 0.0;
    }

    /// Accumulate look mouse deltas while open (OS: x right, y down).
    pub fn add_select_px(&mut self, dx: f32, dy: f32) {
        if !self.open {
            return;
        }
        self.select_x += dx * SELECT_PX_SCALE;
        self.select_y -= dy * SELECT_PX_SCALE;
        let r = (self.select_x * self.select_x + self.select_y * self.select_y).sqrt();
        if r > 1.5 {
            self.select_x *= 1.5 / r;
            self.select_y *= 1.5 / r;
        }
    }

    pub fn highlighted_slot(&self) -> Option<u8> {
        if !self.open {
            return None;
        }
        emote_slot_from_select(self.select_x, self.select_y, EMOTE_WHEEL_DEADZONE)
    }
}

/// Clip-space radial draw (reticle-style alpha blend, no depth write).
pub struct EmoteWheelGpu {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl EmoteWheelGpu {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("emote-wheel"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("emote-wheel-pl"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("emote-wheel-pipeline"),
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

        // Wedges + rings + labels headroom.
        const MAX_VERTS: usize = 4096;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("emote-wheel-verts"),
            size: (MAX_VERTS * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            vertex_count: 0,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, open: bool, highlight: Option<u8>, aspect: f32) {
        if !open {
            self.vertex_count = 0;
            return;
        }

        let aspect = aspect.max(0.25);
        // NDC circle: y free, x compressed by 1/aspect so the wheel is round on screen.
        let sx = 1.0 / aspect;
        let sy = 1.0;

        let mut verts = Vec::with_capacity(1024);

        // Soft dim disc behind the wheel.
        disc_ndc(
            &mut verts,
            sx,
            sy,
            R_OUTER * 1.08,
            [0.04, 0.05, 0.07, 0.42],
            32,
        );

        let n = EMOTE_CATALOG.len() as u32;
        let sector = std::f32::consts::TAU / n as f32;
        for i in 0..n {
            let a0 = i as f32 * sector - sector * 0.5;
            let a1 = a0 + sector;
            let on = highlight == Some(i as u8);
            let fill = if on {
                [0.28, 0.55, 0.88, 0.78]
            } else {
                [0.12, 0.15, 0.20, 0.62]
            };
            annulus_sector_ndc(&mut verts, sx, sy, R_INNER, R_OUTER, a0, a1, fill, ARC_SEGS);

            // Separator spokes.
            let spoke = [0.55, 0.60, 0.70, 0.35];
            thin_radial_ndc(&mut verts, sx, sy, R_INNER, R_OUTER, a0, spoke, 0.008);
        }

        // Outer + inner rings.
        ring_ndc(
            &mut verts,
            sx,
            sy,
            R_OUTER * 0.985,
            R_OUTER,
            [0.75, 0.80, 0.90, 0.55],
            48,
        );
        ring_ndc(
            &mut verts,
            sx,
            sy,
            R_INNER * 0.88,
            R_INNER,
            [0.55, 0.60, 0.70, 0.50],
            32,
        );

        // Labels at wedge midpoints.
        for (i, def) in EMOTE_CATALOG.iter().enumerate() {
            let mid = i as f32 * sector;
            // Angle 0 = +Y, clockwise (match select math).
            let (nx, ny) = ang_to_ndc(mid);
            let cx = nx * R_LABEL * sx;
            let cy = ny * R_LABEL * sy;
            let on = highlight == Some(i as u8);
            let color = if on {
                [1.0, 1.0, 1.0, 0.98]
            } else {
                [0.88, 0.90, 0.95, 0.90]
            };
            // Keep glyphs inside the annulus — 0.028 spilled past wedge edges.
            text_ndc(&mut verts, def.label, cx, cy, 0.014 * sy, sx / sy, color);
        }

        // Cursor tick toward current select (optional subtle).
        // (left out — highlight wedges are enough)

        self.vertex_count = verts.len() as u32;
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.vertex_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}

/// Angle 0 = +Y (up), clockwise positive → unit vector in math NDC (y up).
fn ang_to_ndc(ang: f32) -> (f32, f32) {
    (ang.sin(), ang.cos())
}

fn disc_ndc(out: &mut Vec<Vertex>, sx: f32, sy: f32, r: f32, color: [f32; 4], n: u32) {
    for i in 0..n {
        let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / n as f32) * std::f32::consts::TAU;
        let (x0, y0) = ang_to_ndc(a0);
        let (x1, y1) = ang_to_ndc(a1);
        tri(
            out,
            [0.0, 0.0, 0.0],
            [x0 * r * sx, y0 * r * sy, 0.0],
            [x1 * r * sx, y1 * r * sy, 0.0],
            color,
        );
    }
}

fn ring_ndc(out: &mut Vec<Vertex>, sx: f32, sy: f32, r0: f32, r1: f32, color: [f32; 4], n: u32) {
    for i in 0..n {
        let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / n as f32) * std::f32::consts::TAU;
        let (x0, y0) = ang_to_ndc(a0);
        let (x1, y1) = ang_to_ndc(a1);
        let i0 = [x0 * r0 * sx, y0 * r0 * sy, 0.0];
        let i1 = [x1 * r0 * sx, y1 * r0 * sy, 0.0];
        let o0 = [x0 * r1 * sx, y0 * r1 * sy, 0.0];
        let o1 = [x1 * r1 * sx, y1 * r1 * sy, 0.0];
        tri(out, i0, o0, o1, color);
        tri(out, i0, o1, i1, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn annulus_sector_ndc(
    out: &mut Vec<Vertex>,
    sx: f32,
    sy: f32,
    r0: f32,
    r1: f32,
    a0: f32,
    a1: f32,
    color: [f32; 4],
    segs: u32,
) {
    let segs = segs.max(1);
    for i in 0..segs {
        let t0 = i as f32 / segs as f32;
        let t1 = (i + 1) as f32 / segs as f32;
        let ang0 = a0 + (a1 - a0) * t0;
        let ang1 = a0 + (a1 - a0) * t1;
        let (x0, y0) = ang_to_ndc(ang0);
        let (x1, y1) = ang_to_ndc(ang1);
        let i0 = [x0 * r0 * sx, y0 * r0 * sy, 0.0];
        let i1 = [x1 * r0 * sx, y1 * r0 * sy, 0.0];
        let o0 = [x0 * r1 * sx, y0 * r1 * sy, 0.0];
        let o1 = [x1 * r1 * sx, y1 * r1 * sy, 0.0];
        tri(out, i0, o0, o1, color);
        tri(out, i0, o1, i1, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn thin_radial_ndc(
    out: &mut Vec<Vertex>,
    sx: f32,
    sy: f32,
    r0: f32,
    r1: f32,
    ang: f32,
    color: [f32; 4],
    half_w: f32,
) {
    let (nx, ny) = ang_to_ndc(ang);
    // Perp in NDC before aspect: rotate 90°.
    let px = -ny;
    let py = nx;
    let i0 = [
        (nx * r0 - px * half_w) * sx,
        (ny * r0 - py * half_w) * sy,
        0.0,
    ];
    let i1 = [
        (nx * r0 + px * half_w) * sx,
        (ny * r0 + py * half_w) * sy,
        0.0,
    ];
    let o0 = [
        (nx * r1 - px * half_w) * sx,
        (ny * r1 - py * half_w) * sy,
        0.0,
    ];
    let o1 = [
        (nx * r1 + px * half_w) * sx,
        (ny * r1 + py * half_w) * sy,
        0.0,
    ];
    tri(out, i0, o0, o1, color);
    tri(out, i0, o1, i1, color);
}

fn tri(out: &mut Vec<Vertex>, a: [f32; 3], b: [f32; 3], c: [f32; 3], color: [f32; 4]) {
    out.push(Vertex { position: a, color });
    out.push(Vertex { position: b, color });
    out.push(Vertex { position: c, color });
}

/// 5×7 uppercase bitmap font (bits row-major, top row first, MSB left).
fn glyph5x7(c: char) -> Option<[u8; 7]> {
    // Only the labels we need: A B E N O S V W Y (and space).
    Some(match c {
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        _ => return None,
    })
}

/// Draw a short uppercase label centred at (cx, cy) in NDC.
/// `cell` is pixel height in NDC-y; `aspect_xy` is sx/sy so cells stay square on screen.
fn text_ndc(
    out: &mut Vec<Vertex>,
    text: &str,
    cx: f32,
    cy: f32,
    cell: f32,
    aspect_xy: f32,
    color: [f32; 4],
) {
    let chars: Vec<char> = text
        .chars()
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| glyph5x7(*c).is_some())
        .collect();
    if chars.is_empty() {
        return;
    }
    let cell_x = cell * aspect_xy;
    let cell_y = cell;
    let gap = cell_x * 0.25;
    let char_w = cell_x * 5.0;
    let total_w = chars.len() as f32 * char_w + (chars.len().saturating_sub(1) as f32) * gap;
    let total_h = cell_y * 7.0;
    let mut x0 = cx - total_w * 0.5;
    let y0 = cy + total_h * 0.5;

    for ch in chars {
        let Some(rows) = glyph5x7(ch) else {
            continue;
        };
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..5 {
                if bits & (0b10000 >> col) != 0 {
                    let x = x0 + col as f32 * cell_x;
                    let y = y0 - row as f32 * cell_y;
                    // Pixel quad (y down in row order, NDC y up).
                    let x1 = x + cell_x * 0.9;
                    let y1 = y - cell_y * 0.9;
                    tri(out, [x, y, 0.0], [x1, y, 0.0], [x1, y1, 0.0], color);
                    tri(out, [x, y, 0.0], [x1, y1, 0.0], [x, y1, 0.0], color);
                }
            }
        }
        x0 += char_w + gap;
    }
}
