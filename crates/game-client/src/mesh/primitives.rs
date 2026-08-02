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
    Sides,
    Front,
    Rear,
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
            [z * s, x * s]
        } else {
            [x * s, y * s]
        };
    }
}

/// One door image per leaf: U 0..2 across width, V 0..1 top→bottom (087).
pub fn assign_rear_door_uvs(verts: &mut [MeshVertex]) {
    let min_x = verts
        .iter()
        .map(|v| v.position[0])
        .fold(f32::INFINITY, f32::min);
    let max_x = verts
        .iter()
        .map(|v| v.position[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = verts
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = verts
        .iter()
        .map(|v| v.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let width = (max_x - min_x).max(1e-4);
    let height = (max_y - min_y).max(1e-4);
    for v in verts {
        v.uv = [
            (v.position[0] - min_x) / width * 2.0,
            (max_y - v.position[1]) / height,
        ];
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
        ([0.0, 0.0, -1.0], [0, 1, 2, 3], BoxFaceGroup::Front),
        ([0.0, 0.0, 1.0], [4, 7, 6, 5], BoxFaceGroup::Rear),
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

/// Capped cylinder along local Y (087).
pub fn cylinder_y_prim(radius: f32, half_height: f32, segments: u32, color: [f32; 4]) -> CpuPrim {
    let segments = segments.max(3);
    let radius = radius.max(0.0);
    let half_height = half_height.max(0.0);
    let mut verts = Vec::with_capacity((segments * 4 + 2) as usize);
    let mut indices = Vec::with_capacity((segments * 12) as usize);

    for i in 0..segments {
        let angle = std::f32::consts::TAU * i as f32 / segments as f32;
        let (sin, cos) = angle.sin_cos();
        let normal = [sin, 0.0, cos];
        for y in [-half_height, half_height] {
            verts.push(MeshVertex {
                position: [sin * radius, y, cos * radius],
                normal,
                uv: [
                    i as f32 / segments as f32,
                    (y + half_height) / (2.0 * half_height).max(1e-4),
                ],
            });
        }
    }
    for i in 0..segments {
        let next = (i + 1) % segments;
        let a = i * 2;
        let b = next * 2;
        indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
    }

    for (y, normal, reverse) in [
        (-half_height, [0.0, -1.0, 0.0], true),
        (half_height, [0.0, 1.0, 0.0], false),
    ] {
        let center = verts.len() as u32;
        verts.push(MeshVertex {
            position: [0.0, y, 0.0],
            normal,
            uv: [0.5, 0.5],
        });
        let rim = verts.len() as u32;
        for i in 0..segments {
            let angle = std::f32::consts::TAU * i as f32 / segments as f32;
            let (sin, cos) = angle.sin_cos();
            verts.push(MeshVertex {
                position: [sin * radius, y, cos * radius],
                normal,
                uv: [sin * 0.5 + 0.5, cos * 0.5 + 0.5],
            });
        }
        for i in 0..segments {
            let next = (i + 1) % segments;
            if reverse {
                indices.extend_from_slice(&[center, rim + next, rim + i]);
            } else {
                indices.extend_from_slice(&[center, rim + i, rim + next]);
            }
        }
    }

    (verts, indices, color)
}

/// Hinge strap: pin at local −X (wide in Y), tip at +X (narrow), thickness ±Z.
pub fn hinge_strap_prim(
    half_len: f32,
    half_y_pin: f32,
    half_y_tip: f32,
    half_thick: f32,
    color: [f32; 4],
) -> CpuPrim {
    let hl = half_len.max(1e-4);
    let hyp = half_y_pin.max(0.0);
    let hyt = half_y_tip.max(0.0);
    let ht = half_thick.max(0.0);
    let pin_lo = [-hl, -hyp, -ht];
    let pin_hi = [-hl, hyp, -ht];
    let tip_lo = [hl, -hyt, -ht];
    let tip_hi = [hl, hyt, -ht];
    let pin_lo_z = [-hl, -hyp, ht];
    let pin_hi_z = [-hl, hyp, ht];
    let tip_lo_z = [hl, -hyt, ht];
    let tip_hi_z = [hl, hyt, ht];

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
    let mut push_quad = |a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]| {
        push_tri(a, b, c);
        push_tri(a, c, d);
    };

    push_quad(pin_lo, tip_lo, tip_hi, pin_hi);
    push_quad(pin_lo_z, pin_hi_z, tip_hi_z, tip_lo_z);
    push_quad(pin_lo, pin_hi, pin_hi_z, pin_lo_z);
    push_quad(tip_lo, tip_lo_z, tip_hi_z, tip_hi);
    push_quad(pin_hi, tip_hi, tip_hi_z, pin_hi_z);
    push_quad(pin_lo, pin_lo_z, tip_lo_z, tip_lo);

    (verts, indices, color)
}

pub fn merge_transformed_prims(parts: Vec<(CpuPrim, Mat4)>, color: [f32; 4]) -> CpuPrim {
    let mut verts = Vec::new();
    let mut indices = Vec::new();
    for ((mut part_verts, part_indices, _), transform) in parts {
        let base = verts.len() as u32;
        for vertex in &mut part_verts {
            transform_vertex(vertex, transform);
        }
        verts.extend(part_verts);
        indices.extend(part_indices.into_iter().map(|index| base + index));
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
    fn rear_door_uvs_repeat_one_leaf_twice() {
        let (mut rear, _, _) = box_face_group_prim(
            glam::Vec3::new(1.2, 1.3, 3.0),
            [1.0; 4],
            Some(BoxFaceGroup::Rear),
        );
        assign_rear_door_uvs(&mut rear);
        assert!(rear.iter().any(|v| v.uv == [0.0, 1.0]));
        assert!(rear.iter().any(|v| v.uv == [0.0, 0.0]));
        assert!(rear.iter().any(|v| v.uv == [2.0, 0.0]));
        assert!(rear.iter().any(|v| v.uv == [2.0, 1.0]));
    }

    #[test]
    fn box_face_group_emits_only_requested_faces() {
        let half = glam::Vec3::new(1.0, 1.0, 1.0);
        let (sides, _, _) = box_face_group_prim(half, [1.0; 4], Some(BoxFaceGroup::Sides));
        assert_eq!(sides.len(), 8);
        assert!(sides.iter().all(|v| v.normal[0].abs() > 0.5));

        let (front, _, _) = box_face_group_prim(half, [1.0; 4], Some(BoxFaceGroup::Front));
        assert_eq!(front.len(), 4);
        assert!(front.iter().all(|v| v.normal[2] < -0.5));

        let (rear, _, _) = box_face_group_prim(half, [1.0; 4], Some(BoxFaceGroup::Rear));
        assert_eq!(rear.len(), 4);
        assert!(rear.iter().all(|v| v.normal[2] > 0.5));

        let (lids, _, _) = box_face_group_prim(half, [1.0; 4], Some(BoxFaceGroup::Lids));
        assert_eq!(lids.len(), 8);
        assert!(lids.iter().all(|v| v.normal[1].abs() > 0.5));
    }

    #[test]
    fn merge_transformed_prims_offsets_indices_and_positions() {
        let color = [1.0; 4];
        let merged = merge_transformed_prims(
            vec![
                (
                    box_prim(glam::Vec3::splat(0.5), color),
                    Mat4::from_translation(Vec3::X),
                ),
                (
                    cylinder_y_prim(0.1, 0.5, 8, color),
                    Mat4::from_translation(Vec3::Y),
                ),
            ],
            color,
        );
        assert_eq!(merged.1.len(), 36 + 8 * 12);
        assert!(merged.0.iter().any(|v| v.position[0] > 1.4));
        assert!(merged.1.iter().all(|&i| i < merged.0.len() as u32));
    }
}
