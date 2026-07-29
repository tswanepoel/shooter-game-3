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

pub fn box_prim(half: glam::Vec3, color: [f32; 4]) -> CpuPrim {
    let hx = half.x;
    let hy = half.y;
    let hz = half.z;
    let corners = [
        [-hx, -hy, -hz],
        [hx, -hy, -hz],
        [hx, hy, -hz],
        [-hx, hy, -hz],
        [-hx, -hy, hz],
        [hx, -hy, hz],
        [hx, hy, hz],
        [-hx, hy, hz],
    ];
    let faces: [([f32; 3], [usize; 4]); 6] = [
        ([0.0, 0.0, -1.0], [0, 1, 2, 3]),
        ([0.0, 0.0, 1.0], [4, 7, 6, 5]),
        ([-1.0, 0.0, 0.0], [0, 3, 7, 4]),
        ([1.0, 0.0, 0.0], [1, 5, 6, 2]),
        ([0.0, -1.0, 0.0], [0, 4, 5, 1]),
        ([0.0, 1.0, 0.0], [3, 2, 6, 7]),
    ];
    let mut verts = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (normal, quad) in faces {
        let base = verts.len() as u32;
        for i in quad {
            verts.push(MeshVertex {
                position: corners[i],
                normal,
                uv: [0.0, 0.0],
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, indices, color)
}

/// Wedge ramp: footprint `±half_x` × `±half_z`, top rises from y=0 at −z to `height` at +z.
pub fn ramp_prim(half_x: f32, half_z: f32, height: f32, color: [f32; 4]) -> CpuPrim {
    let hx = half_x;
    let hz = half_z;
    let h = height.max(0.0);
    // Bottom rectangle + sloping top + sides.
    let bl = [-hx, 0.0, -hz];
    let br = [hx, 0.0, -hz];
    let fl = [-hx, 0.0, hz];
    let fr = [hx, 0.0, hz];
    let ull = [-hx, h, hz];
    let ur = [hx, h, hz];

    let mut verts = Vec::new();
    let mut indices = Vec::new();
    let mut push_tri = |a: [f32; 3], b: [f32; 3], c: [f32; 3]| {
        let ab = Vec3::from_array(b) - Vec3::from_array(a);
        let ac = Vec3::from_array(c) - Vec3::from_array(a);
        let n = ab.cross(ac);
        let n = if n.length_squared() > 1e-12 {
            n.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };
        let base = verts.len() as u32;
        for p in [a, b, c] {
            verts.push(MeshVertex {
                position: p,
                normal: n,
                uv: [0.0, 0.0],
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    };

    // Bottom (y=0), winding so normal −Y.
    push_tri(bl, fl, br);
    push_tri(br, fl, fr);
    // Sloped top (low end at y=0, high end at y=h).
    push_tri(bl, br, ur);
    push_tri(bl, ur, ull);
    // High end vertical.
    push_tri(fl, ull, fr);
    push_tri(fr, ull, ur);
    // −X side.
    push_tri(bl, ull, fl);
    // +X side.
    push_tri(br, fr, ur);

    (verts, indices, color)
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
