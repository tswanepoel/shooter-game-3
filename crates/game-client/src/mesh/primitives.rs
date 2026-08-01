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

/// Assign UVs from world XZ metres so albedo tiles in place (083).
pub fn assign_world_xz_uvs(verts: &mut [MeshVertex], metres_per_tile: f32) {
    let s = 1.0 / metres_per_tile.max(1e-4);
    for v in verts {
        v.uv = [v.position[0] * s, v.position[2] * s];
    }
}

/// Which faces of a box solid to emit (086 multi-material container).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxFaceGroup {
    /// ±X long walls.
    Sides,
    /// ±Z ends (doors).
    Ends,
    /// ±Y roof / floor.
    Lids,
}

/// Assign UVs from local face metres so corrugation stays upright on box sides (086).
/// Call before transforming verts into world space. `v` follows local +Y on vertical faces.
/// On lids, UVs are transposed so side-albedo grooves run width-wise (±X).
pub fn assign_box_face_uvs(verts: &mut [MeshVertex], metres_per_tile: f32) {
    let s = 1.0 / metres_per_tile.max(1e-4);
    for v in verts {
        let [x, y, z] = v.position;
        let [nx, ny, nz] = v.normal;
        let ax = nx.abs();
        let ay = ny.abs();
        let az = nz.abs();
        v.uv = if ax >= ay && ax >= az {
            [z * s, y * s]
        } else if ay >= ax && ay >= az {
            // Transpose vs [x, z]: rotate side tiles 90° so grooves run along width.
            [z * s, x * s]
        } else {
            [x * s, y * s]
        };
    }
}

pub fn box_prim(half: glam::Vec3, color: [f32; 4]) -> CpuPrim {
    box_face_group_prim(half, color, None)
}

/// Box faces for one material group. `None` emits all six faces.
pub fn box_face_group_prim(
    half: glam::Vec3,
    color: [f32; 4],
    group: Option<BoxFaceGroup>,
) -> CpuPrim {
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
    // (normal, quad, group)
    let faces: [([f32; 3], [usize; 4], BoxFaceGroup); 6] = [
        ([0.0, 0.0, -1.0], [0, 1, 2, 3], BoxFaceGroup::Ends),
        ([0.0, 0.0, 1.0], [4, 7, 6, 5], BoxFaceGroup::Ends),
        ([-1.0, 0.0, 0.0], [0, 3, 7, 4], BoxFaceGroup::Sides),
        ([1.0, 0.0, 0.0], [1, 5, 6, 2], BoxFaceGroup::Sides),
        ([0.0, -1.0, 0.0], [0, 4, 5, 1], BoxFaceGroup::Lids),
        ([0.0, 1.0, 0.0], [3, 2, 6, 7], BoxFaceGroup::Lids),
    ];
    let mut verts = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (normal, quad, face_group) in faces {
        if group.is_some_and(|g| g != face_group) {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_xz_uvs_from_metres() {
        let mut verts = [MeshVertex {
            position: [3.0, 0.0, -1.5],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        }];
        assign_world_xz_uvs(&mut verts, 1.5);
        assert!((verts[0].uv[0] - 2.0).abs() < 1e-5);
        assert!((verts[0].uv[1] - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn box_face_uvs_upright_on_sides() {
        let mut side = [MeshVertex {
            position: [1.0, 0.5, -0.25],
            normal: [1.0, 0.0, 0.0],
            uv: [0.0, 0.0],
        }];
        assign_box_face_uvs(&mut side, 1.0);
        assert!((side[0].uv[0] - (-0.25)).abs() < 1e-5);
        assert!((side[0].uv[1] - 0.5).abs() < 1e-5);

        let mut end = [MeshVertex {
            position: [-0.5, 1.0, 2.0],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
        }];
        assign_box_face_uvs(&mut end, 1.0);
        assert!((end[0].uv[0] - (-0.5)).abs() < 1e-5);
        assert!((end[0].uv[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn box_face_uvs_lid_grooves_widthwise() {
        let mut lid = [MeshVertex {
            position: [0.4, 1.0, -0.8],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        }];
        assign_box_face_uvs(&mut lid, 1.0);
        assert!((lid[0].uv[0] - (-0.8)).abs() < 1e-5);
        assert!((lid[0].uv[1] - 0.4).abs() < 1e-5);
    }

    #[test]
    fn box_face_group_emits_only_requested_faces() {
        let half = glam::Vec3::new(1.0, 1.0, 1.0);
        let (sides, _, _) = box_face_group_prim(half, [1.0; 4], Some(BoxFaceGroup::Sides));
        assert_eq!(sides.len(), 8);
        assert!(sides.iter().all(|v| v.normal[0].abs() > 0.5));

        let (ends, _, _) = box_face_group_prim(half, [1.0; 4], Some(BoxFaceGroup::Ends));
        assert_eq!(ends.len(), 8);
        assert!(ends.iter().all(|v| v.normal[2].abs() > 0.5));

        let (lids, _, _) = box_face_group_prim(half, [1.0; 4], Some(BoxFaceGroup::Lids));
        assert_eq!(lids.len(), 8);
        assert!(lids.iter().all(|v| v.normal[1].abs() > 0.5));
    }
}
