use glam::Vec3;

/// Key + ambient plate for the lit matte path (**018**).
#[derive(Clone, Copy, Debug)]
pub struct LightPlate {
    /// Direction **toward** the key light (world space); normalized on write.
    pub light_dir: Vec3,
    /// Key contribution at N·L wrap = 1 (display-referred multiply).
    pub key_color: [f32; 3],
    /// Ambient fill so unlit sides stay readable.
    pub ambient: [f32; 3],
}

/// Default plate for lineup / non-map present (**018**).
pub const DEFAULT_LIGHT_PLATE: LightPlate = LightPlate {
    light_dir: Vec3::new(0.45, 0.82, 0.35),
    key_color: [0.70, 0.70, 0.68],
    ambient: [0.42, 0.42, 0.44],
};

pub(crate) const KIT_SHADER: &str = r#"
struct FrameUniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec3<f32>,
    _pad0: f32,
    key_color: vec3<f32>,
    _pad1: f32,
    ambient: vec3<f32>,
    _pad2: f32,
};

struct MaterialUniforms {
    base_color: vec4<f32>,
    // x: 1 = lit (kit mesh, map solid), 0 = unlit debug. (vec4 for uniform alignment)
    flags: vec4<f32>,
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
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = frame.view_proj * vec4<f32>(in.position, 1.0);
    out.normal = in.normal;
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(
    in: VertexOutput,
    @builtin(front_facing) front_facing: bool,
) -> @location(0) vec4<f32> {
    let tex = textureSample(albedo, albedo_samp, in.uv);
    let albedo = tex * material.base_color;

    if (material.flags.x < 0.5) {
        return albedo;
    }

    // Double-sided: flip N on backfaces so lighting does not invert.
    var n = normalize(in.normal);
    if (!front_facing) {
        n = -n;
    }
    // Half-Lambert (Valve-style wrap): softens the lit/dark edge on blocky kits.
    let ndotl = dot(n, normalize(frame.light_dir));
    let wrap = ndotl * 0.5 + 0.5;
    let diffuse = wrap * wrap;
    let lighting = frame.ambient + frame.key_color * diffuse;
    return vec4<f32>(albedo.rgb * lighting, albedo.a);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct FrameUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 3],
    pub _pad0: f32,
    pub key_color: [f32; 3],
    pub _pad1: f32,
    pub ambient: [f32; 3],
    pub _pad2: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct MaterialUniforms {
    pub base_color: [f32; 4],
    /// x: 1 = lit, 0 = unlit debug.
    pub flags: [f32; 4],
}
