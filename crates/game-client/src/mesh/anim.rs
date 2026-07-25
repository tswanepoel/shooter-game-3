use glam::{Mat4, Quat, Vec3};

use super::gltf::CharPart;
use super::kit::HOLDING_RIGHT_ROT;

/// glTF animation path on a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimPath {
    Translation,
    Rotation,
    Scale,
}

/// One channel of a named clip (LINEAR keys).
#[derive(Clone)]
pub struct AnimChannel {
    pub node: String,
    pub path: AnimPath,
    pub times: Vec<f32>,
    /// Flat key values: 3 floats/key (T/S) or 4 (quat xyzw).
    pub values: Vec<f32>,
}

/// Named glTF clip for rigid node TRS (Kenney character kit).
#[derive(Clone)]
pub struct AnimClip {
    pub duration: f32,
    pub channels: Vec<AnimChannel>,
}

/// Sampled walk overrides for one node (only fields present in the clip).
#[derive(Clone, Copy, Default)]
struct NodeAnimSample {
    translation: Option<Vec3>,
    rotation: Option<Quat>,
    scale: Option<Vec3>,
}

impl AnimClip {
    /// Sample sparse local TRS overrides at phase ∈ [0, 1).
    fn sample_overrides(&self, phase: f32) -> std::collections::HashMap<String, NodeAnimSample> {
        let t = phase.rem_euclid(1.0) * self.duration.max(1e-8);
        self.sample_overrides_at(t)
    }

    /// Sample at absolute clip time (seconds), clamped to the clip range (one-shot emotes).
    fn sample_overrides_at(
        &self,
        time_s: f32,
    ) -> std::collections::HashMap<String, NodeAnimSample> {
        use std::collections::HashMap;
        let t = time_s.clamp(0.0, self.duration.max(0.0));
        let mut out: HashMap<String, NodeAnimSample> = HashMap::new();

        for ch in &self.channels {
            let entry = out.entry(ch.node.clone()).or_default();
            match ch.path {
                AnimPath::Translation => {
                    entry.translation = sample_vec3(&ch.times, &ch.values, t);
                }
                AnimPath::Rotation => {
                    entry.rotation = sample_quat(&ch.times, &ch.values, t);
                }
                AnimPath::Scale => {
                    entry.scale = sample_vec3(&ch.times, &ch.values, t);
                }
            }
        }
        out
    }
}

fn apply_anim_to_bind(bind: Mat4, sample: NodeAnimSample) -> Mat4 {
    let (bind_scale, bind_rot, bind_trans) = bind.to_scale_rotation_translation();
    // Prefer column lengths when decomposition is unstable on near-identity.
    let scale = sample.scale.unwrap_or_else(|| {
        Vec3::new(
            bind.x_axis.truncate().length(),
            bind.y_axis.truncate().length(),
            bind.z_axis.truncate().length(),
        )
    });
    let rot = sample.rotation.unwrap_or(bind_rot);
    let trans = sample.translation.unwrap_or(bind_trans);
    let _ = bind_scale;
    Mat4::from_scale_rotation_translation(scale, rot, trans)
}

fn sample_vec3(times: &[f32], values: &[f32], t: f32) -> Option<Vec3> {
    let (i0, i1, a) = key_span(times, t)?;
    let a0 = Vec3::new(values[i0 * 3], values[i0 * 3 + 1], values[i0 * 3 + 2]);
    let a1 = Vec3::new(values[i1 * 3], values[i1 * 3 + 1], values[i1 * 3 + 2]);
    Some(a0.lerp(a1, a))
}

fn sample_quat(times: &[f32], values: &[f32], t: f32) -> Option<Quat> {
    let (i0, i1, a) = key_span(times, t)?;
    let q0 = Quat::from_xyzw(
        values[i0 * 4],
        values[i0 * 4 + 1],
        values[i0 * 4 + 2],
        values[i0 * 4 + 3],
    )
    .normalize();
    let q1 = Quat::from_xyzw(
        values[i1 * 4],
        values[i1 * 4 + 1],
        values[i1 * 4 + 2],
        values[i1 * 4 + 3],
    )
    .normalize();
    Some(q0.slerp(q1, a))
}

fn key_span(times: &[f32], t: f32) -> Option<(usize, usize, f32)> {
    if times.is_empty() {
        return None;
    }
    if times.len() == 1 {
        return Some((0, 0, 0.0));
    }
    let t = t.clamp(times[0], *times.last().unwrap());
    let mut i1 = 1;
    while i1 < times.len() && times[i1] < t {
        i1 += 1;
    }
    let i0 = i1.saturating_sub(1);
    let i1 = i1.min(times.len() - 1);
    let span = times[i1] - times[i0];
    let a = if span > 1e-8 {
        (t - times[i0]) / span
    } else {
        0.0
    };
    Some((i0, i1, a.clamp(0.0, 1.0)))
}

/// Extract a named LINEAR clip from a character GLB.
pub fn extract_clip(glb: &[u8], name: &str) -> Result<AnimClip, String> {
    let gltf = gltf::Gltf::from_slice(glb).map_err(|e| format!("gltf parse: {e}"))?;
    let blob = gltf
        .blob
        .as_ref()
        .ok_or_else(|| "GLB missing BIN chunk".to_string())?;

    let anim = gltf
        .animations()
        .find(|a| a.name() == Some(name))
        .ok_or_else(|| format!("clip '{name}' not found"))?;

    let mut channels = Vec::new();
    let mut duration = 0.0_f32;

    for channel in anim.channels() {
        let target = channel.target();
        let node_name = target.node().name().unwrap_or("").to_string();
        let path = match target.property() {
            gltf::animation::Property::Translation => AnimPath::Translation,
            gltf::animation::Property::Rotation => AnimPath::Rotation,
            gltf::animation::Property::Scale => AnimPath::Scale,
            gltf::animation::Property::MorphTargetWeights => continue,
        };

        let reader = channel.reader(|buffer| {
            if buffer.index() == 0 {
                Some(blob.as_slice())
            } else {
                None
            }
        });

        let times: Vec<f32> = reader
            .read_inputs()
            .ok_or_else(|| format!("clip '{name}' channel missing times"))?
            .collect();
        if let Some(&last) = times.last() {
            duration = duration.max(last);
        }

        let values: Vec<f32> = match path {
            AnimPath::Translation | AnimPath::Scale => {
                let outputs = reader
                    .read_outputs()
                    .ok_or_else(|| format!("clip '{name}' channel missing outputs"))?;
                match outputs {
                    gltf::animation::util::ReadOutputs::Translations(iter) => {
                        iter.flat_map(|v| [v[0], v[1], v[2]]).collect()
                    }
                    gltf::animation::util::ReadOutputs::Scales(iter) => {
                        iter.flat_map(|v| [v[0], v[1], v[2]]).collect()
                    }
                    _ => return Err(format!("clip '{name}' unexpected output for {path:?}")),
                }
            }
            AnimPath::Rotation => {
                let outputs = reader
                    .read_outputs()
                    .ok_or_else(|| format!("clip '{name}' channel missing outputs"))?;
                match outputs {
                    gltf::animation::util::ReadOutputs::Rotations(rots) => rots
                        .into_f32()
                        .flat_map(|v| [v[0], v[1], v[2], v[3]])
                        .collect(),
                    _ => return Err(format!("clip '{name}' unexpected output for rotation")),
                }
            }
        };

        channels.push(AnimChannel {
            node: node_name,
            path,
            times,
            values,
        });
    }

    if channels.is_empty() {
        return Err(format!("clip '{name}' has no channels"));
    }
    if duration <= 0.0 {
        duration = game_sim::WALK_CLIP_DURATION_S;
    }

    Ok(AnimClip { duration, channels })
}

/// Pose character parts from sim drive. Returns kit-space worlds and arm-right.
pub fn pose_character_kit(
    parts: &[CharPart],
    self_state: &game_sim::SelfState,
    loco_clip: Option<&AnimClip>,
    emote_clip: Option<(&AnimClip, f32)>,
    die_clip: Option<(&AnimClip, f32)>,
) -> (Vec<Mat4>, Mat4) {
    let dying = !self_state.alive && die_clip.is_some();
    let emoting = !dying && emote_clip.is_some() && self_state.is_emoting();
    let sprinting = self_state.locomotion.is_sprint() && !emoting && !dying;
    let armed = self_state.presents_armed() && !dying;
    // Hold + aim owns the right arm only while armed and not sprinting / emoting.
    let hold_right = armed && !sprinting && !emoting;
    let die_over = match die_clip {
        Some((clip, age)) if dying => Some(clip.sample_overrides_at(age)),
        _ => None,
    };
    let loco_over = match loco_clip {
        Some(clip) if !dying && !emoting && self_state.locomotion.uses_loco_clip() => {
            Some(clip.sample_overrides(self_state.walk_phase))
        }
        _ => None,
    };
    let emote_over = match emote_clip {
        Some((clip, age)) if emoting => Some(clip.sample_overrides_at(age)),
        _ => None,
    };

    let mut locals = Vec::with_capacity(parts.len());
    for p in parts {
        let mut local = p.bind_local;

        // Full-body collapse owns every channel the kit authored (root + limbs).
        if let Some(ref over) = die_over {
            if let Some(sample) = over.get(&p.name) {
                local = apply_anim_to_bind(p.bind_local, *sample);
            }
            locals.push(local);
            continue;
        }

        if let Some(ref over) = loco_over {
            if let Some(sample) = over.get(&p.name) {
                // Walk armed: legs + left arm (right stays hold). Sprint or unarmed: both arms.
                let apply_loco = if sprinting || !armed {
                    matches!(
                        p.name.as_str(),
                        "root" | "leg-left" | "leg-right" | "arm-left" | "arm-right"
                    )
                } else {
                    matches!(
                        p.name.as_str(),
                        "root" | "leg-left" | "leg-right" | "arm-left"
                    )
                };
                if apply_loco {
                    local = apply_anim_to_bind(p.bind_local, *sample);
                }
            }
        }

        // Emote owns upper-body channels (holster: no hold layer).
        if let Some(ref over) = emote_over {
            if matches!(p.name.as_str(), "arm-left" | "arm-right" | "torso" | "head") {
                if let Some(sample) = over.get(&p.name) {
                    local = apply_anim_to_bind(p.bind_local, *sample);
                }
            }
        }

        match p.name.as_str() {
            "torso" if !sprinting && !emoting => {
                local *= Mat4::from_quat(Quat::from_rotation_x(-self_state.torso_pitch));
            }
            "arm-right" if hold_right => {
                // Armed hold owns the right arm (left keeps walk swing).
                let (_s, _r, t) = local.to_scale_rotation_translation();
                let scale = {
                    let sx = local.x_axis.truncate().length();
                    let sy = local.y_axis.truncate().length();
                    let sz = local.z_axis.truncate().length();
                    Vec3::new(sx, sy, sz)
                };
                local = Mat4::from_scale_rotation_translation(scale, HOLDING_RIGHT_ROT, t)
                    * Mat4::from_quat(Quat::from_rotation_y(self_state.shoulder_yaw))
                    * Mat4::from_quat(Quat::from_rotation_x(-self_state.shoulder_pitch));
            }
            "head" if !emoting => {
                // Look owns head attitude (015); walk head channel is unused for local self.
                let (_s, _r, t) = local.to_scale_rotation_translation();
                let scale = {
                    let sx = local.x_axis.truncate().length();
                    let sy = local.y_axis.truncate().length();
                    let sz = local.z_axis.truncate().length();
                    Vec3::new(sx, sy, sz)
                };
                let head_rot = Quat::from_rotation_y(self_state.head_yaw)
                    * Quat::from_rotation_x(-self_state.head_pitch);
                local = Mat4::from_scale_rotation_translation(scale, head_rot, t);
            }
            _ => {}
        }
        locals.push(local);
    }

    let mut worlds = vec![Mat4::IDENTITY; parts.len()];
    for i in 0..parts.len() {
        worlds[i] = match parts[i].parent {
            Some(pi) => worlds[pi] * locals[i],
            None => locals[i],
        };
    }

    let arm = parts
        .iter()
        .position(|p| p.name == "arm-right")
        .map(|i| worlds[i])
        .unwrap_or(Mat4::IDENTITY);

    (worlds, arm)
}
