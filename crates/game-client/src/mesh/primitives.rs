use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub type CpuPrim = (Vec<MeshVertex>, Vec<u32>, [f32; 4]);

/// Transform position and normal by `m` (normal uses direction only, then normalize).
pub fn transform_vertex(v: &mut MeshVertex, m: Mat4) {
    v.position = m.transform_point3(Vec3::from_array(v.position)).to_array();
    let n = m.transform_vector3(Vec3::from_array(v.normal));
    v.normal = if n.length_squared() > 1e-12 {
        n.normalize().to_array()
    } else {
        [0.0, 1.0, 0.0]
    };
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
            let n = [sx * sy, cy, cx * sy];
            verts.push(MeshVertex {
                position: n,
                normal: n,
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
