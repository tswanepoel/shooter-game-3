//! Flight segment vs posed character part meshes (043).

use game_sim::SelfState;
use glam::{Mat4, Vec3};

use crate::mesh_unlit::{self, AnimClip, CharPart, KitPose, MeshVertex};

#[derive(Debug, Clone)]
pub struct PartHit {
    #[allow(dead_code)]
    pub part: String,
    pub position: Vec3,
    /// Parametric t along `from → to` in [0, 1].
    pub t: f32,
}

#[allow(clippy::too_many_arguments)]
pub fn trace_segment_parts(
    parts: &[CharPart],
    self_state: &SelfState,
    loco: Option<&AnimClip>,
    emote: Option<(&AnimClip, f32)>,
    die: Option<(&AnimClip, f32)>,
    min_y: f32,
    from: Vec3,
    to: Vec3,
) -> Option<PartHit> {
    let k2w = mesh_unlit::kit_to_world(self_state.placement_matrix(), min_y);
    let (worlds, _) =
        mesh_unlit::pose_character_kit(parts, self_state, loco, emote, die, KitPose::Present);

    let mut best: Option<PartHit> = None;
    for (i, part) in parts.iter().enumerate() {
        if part.local_verts.is_empty() || part.indices.len() < 3 {
            continue;
        }
        let world = k2w * worlds[i];
        if let Some(hit) = segment_vs_part_mesh(from, to, part, world) {
            if best.as_ref().map(|b| hit.t < b.t).unwrap_or(true) {
                best = Some(hit);
            }
        }
    }
    best
}

fn segment_vs_part_mesh(from: Vec3, to: Vec3, part: &CharPart, world: Mat4) -> Option<PartHit> {
    let inv = world.inverse();
    let a = inv.transform_point3(from);
    let b = inv.transform_point3(to);
    let dir = b - a;
    let seg_len_sq = dir.length_squared();
    if seg_len_sq < 1e-16 {
        return None;
    }

    let mut best_t = f32::INFINITY;
    let mut best_local = Vec3::ZERO;

    let idxs = &part.indices;
    let verts = &part.local_verts;
    let mut i = 0;
    while i + 2 < idxs.len() {
        let i0 = idxs[i] as usize;
        let i1 = idxs[i + 1] as usize;
        let i2 = idxs[i + 2] as usize;
        i += 3;
        if i0 >= verts.len() || i1 >= verts.len() || i2 >= verts.len() {
            continue;
        }
        let v0 = vert_pos(&verts[i0]);
        let v1 = vert_pos(&verts[i1]);
        let v2 = vert_pos(&verts[i2]);
        if let Some((t, p)) = segment_triangle(a, dir, v0, v1, v2) {
            if t < best_t {
                best_t = t;
                best_local = p;
            }
        }
    }

    if best_t <= 1.0 {
        Some(PartHit {
            part: part.name.clone(),
            position: world.transform_point3(best_local),
            t: best_t,
        })
    } else {
        None
    }
}

fn vert_pos(v: &MeshVertex) -> Vec3 {
    Vec3::from_array(v.position)
}

fn segment_triangle(origin: Vec3, dir: Vec3, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<(f32, Vec3)> {
    const EPS: f32 = 1e-7;
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let pvec = dir.cross(e2);
    let det = e1.dot(pvec);
    if det > -EPS && det < EPS {
        return None;
    }
    let inv_det = 1.0 / det;
    let tvec = origin - v0;
    let u = tvec.dot(pvec) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(e1);
    let v = dir.dot(qvec) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(qvec) * inv_det;
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    Some((t, origin + dir * t))
}

#[cfg(test)]
mod tests {
    use super::segment_triangle;
    use glam::Vec3;

    #[test]
    fn segment_hits_unit_triangle() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let origin = Vec3::new(0.25, 0.25, -1.0);
        let dir = Vec3::new(0.0, 0.0, 2.0);
        let hit = segment_triangle(origin, dir, v0, v1, v2).expect("hit");
        assert!((hit.0 - 0.5).abs() < 1e-4);
    }

    #[test]
    fn segment_misses_beside_triangle() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let origin = Vec3::new(2.0, 2.0, -1.0);
        let dir = Vec3::new(0.0, 0.0, 2.0);
        assert!(segment_triangle(origin, dir, v0, v1, v2).is_none());
    }
}
