use glam::{Mat4, Quat, Vec3};

use super::kit::HOLDING_RIGHT_ROT;
use super::primitives::{transform_vertex, CpuPrim, MeshVertex};

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

fn read_normals<'a, 's, F>(reader: &gltf::mesh::Reader<'a, 's, F>, count: usize) -> Vec<[f32; 3]>
where
    F: Clone + Fn(gltf::Buffer<'a>) -> Option<&'s [u8]>,
{
    match reader.read_normals() {
        Some(iter) => {
            let n: Vec<[f32; 3]> = iter.collect();
            if n.len() == count {
                n
            } else {
                vec![[0.0, 1.0, 0.0]; count]
            }
        }
        None => vec![[0.0, 1.0, 0.0]; count],
    }
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
            let normals = read_normals(&reader, positions.len());
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
                    normal: normals[i],
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
            let normals = read_normals(&reader, positions.len());
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
                let mut v = MeshVertex {
                    position: *p,
                    normal: normals[i],
                    uv: uvs[i],
                };
                transform_vertex(&mut v, world);
                *min_y = min_y.min(v.position[1]);
                verts.push(v);
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
