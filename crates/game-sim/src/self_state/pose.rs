//! Look, body presentation, and joint residual for SelfState.

use glam::{Mat4, Quat, Vec3};

use crate::weapons::WeaponDef;

use super::SelfState;

/// Look elevation hard cap: straight up / straight down (015).
pub const OCULAR_ELEV_CAP_RAD: f32 = std::f32::consts::FRAC_PI_2;

/// Hip look-offset fold at full look-down (inward).
pub const TORSO_PITCH_INWARD_RAD: f32 = 15.0_f32.to_radians();
/// Hip look-offset fold at full look-up (outward).
pub const TORSO_PITCH_OUTWARD_RAD: f32 = 7.5_f32.to_radians();
/// Right-shoulder look-offset fold at full look-down (inward).
pub const SHOULDER_PITCH_INWARD_RAD: f32 = 75.0_f32.to_radians();
/// Right-shoulder look-offset fold at full look-up (outward).
pub const SHOULDER_PITCH_OUTWARD_RAD: f32 = 82.5_f32.to_radians();

const HEAD_PITCH_BUDGET_RAD: f32 = 50.0_f32.to_radians();

const FIRE_CONTINUE_FALL_MULT: f32 = 6.0;

const HIT_FOLD_DEG_PER_DMG: f32 = 0.055;
const HIT_TWIST_DEG_PER_DMG: f32 = 0.022;
const HIT_FOLD_CAP_DEG: f32 = 1.6;
const HIT_TWIST_CAP_DEG: f32 = 0.65;
pub(crate) const HIT_FALL_S: f32 = 0.12;

fn fall_toward_zero(value: f32, dt: f32, fall_s: f32) -> f32 {
    if fall_s <= 1e-6 {
        return 0.0;
    }
    let t = (dt / fall_s).clamp(0.0, 1.0);
    let mut v = value * (1.0 - t);
    if v.abs() < 1e-5 {
        v = 0.0;
    }
    v
}

/// Default range along look for aim markers when max range is unknown (metres).
pub const DEFAULT_BORE_RANGE_M: f32 = 100.0;
/// World depth of the screen-centre reticle billboard (metres).
pub const RETICLE_DEPTH_M: f32 = 4.0;
/// On-screen reticle diameter (CSS/logical px).
pub const RETICLE_SIZE_PX: f32 = 6.0;
/// Head-local face offset in character-kit units (applied under posed `head` node).
///
/// Chosen so rest look origin matches the calibrated feet-local eye
/// `(0, 1.52, 0.27)` m (character-a: head pivot `(0, 1.9, 0)` kit, scale `0.1`,
/// kitΓåÆm `1/1.5`, soles on y = 0).
pub const FACE_OFFSET_HEAD_KIT: Vec3 = Vec3::new(0.0, 3.8, 4.05);

impl SelfState {
    /// Drive look direction (ocular yaw + pitch).
    pub fn ocular_forward(&self) -> Vec3 {
        dir_yaw_pitch(self.ocular_yaw, self.ocular_pitch)
    }

    /// Look direction after hip + neck residual (view rides this).
    pub fn look_forward(&self) -> Vec3 {
        dir_yaw_pitch(self.look_yaw(), self.look_pitch())
    }

    pub fn look_yaw(&self) -> f32 {
        self.ocular_yaw + self.head_yaw
    }

    pub fn look_pitch(&self) -> f32 {
        (self.ocular_pitch
            + self.hip_fire_fold
            + self.hip_hit_fold
            + self.neck_fire_fold
            + self.neck_hit_fold)
            .clamp(-OCULAR_ELEV_CAP_RAD, OCULAR_ELEV_CAP_RAD)
    }

    pub fn fire_fold_total(&self) -> f32 {
        self.hip_fire_fold + self.shoulder_fire_fold + self.neck_fire_fold
    }

    pub fn hit_fold_total(&self) -> f32 {
        self.hip_hit_fold + self.shoulder_hit_fold + self.neck_hit_fold
    }

    /// Horizontal look forward on XZ (unit, y = 0).
    pub fn look_forward_xz(&self) -> Vec3 {
        Vec3::new(self.ocular_yaw.sin(), 0.0, self.ocular_yaw.cos())
    }

    /// Horizontal look right on XZ (unit, y = 0).
    /// Matches RH view / flycam: `forward_xz ├ù world_up` (screen-right).
    pub fn look_right_xz(&self) -> Vec3 {
        let f = self.look_forward_xz();
        Vec3::new(-f.z, 0.0, f.x)
    }

    /// Body facing on XZ from torso yaw.
    pub fn body_facing(&self) -> Vec3 {
        Vec3::new(self.torso_yaw.sin(), 0.0, self.torso_yaw.cos())
    }

    /// Root placement: feet position + absolute body yaw.
    pub fn placement_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_y(self.torso_yaw), self.position)
    }

    /// Apply mouse look deltas (radians) and snap body presentation to look.
    /// `dt` is unused; kept so call sites stay frame-shaped.
    pub fn apply_look(&mut self, _dt: f32, delta_yaw: f32, delta_pitch: f32) {
        self.ocular_yaw += delta_yaw;
        self.ocular_pitch =
            (self.ocular_pitch + delta_pitch).clamp(-OCULAR_ELEV_CAP_RAD, OCULAR_ELEV_CAP_RAD);
        self.sync_pose();
    }

    /// Set absolute look (radians) and snap body presentation. Used by server net apply.
    pub fn set_look(&mut self, yaw: f32, pitch: f32) {
        self.ocular_yaw = yaw;
        self.ocular_pitch = pitch.clamp(-OCULAR_ELEV_CAP_RAD, OCULAR_ELEV_CAP_RAD);
        self.sync_pose();
    }

    pub fn sync_pose(&mut self) {
        self.compose_joints();
    }

    pub fn compose_joints(&mut self) {
        let (hip_look, shoulder_look) = elevation_targets(self.ocular_pitch);
        let neck_look =
            (self.ocular_pitch - hip_look).clamp(-HEAD_PITCH_BUDGET_RAD, HEAD_PITCH_BUDGET_RAD);
        self.torso_yaw = self.ocular_yaw;
        self.torso_pitch = hip_look + self.hip_fire_fold + self.hip_hit_fold;
        self.shoulder_pitch = shoulder_look
            + self.shoulder_fire_fold
            + self.shoulder_hit_fold
            + self.shoulder_sway_fold;
        self.shoulder_yaw =
            self.shoulder_fire_twist + self.shoulder_hit_twist + self.shoulder_sway_twist;
        // Facing tracks look: no neck look-offset twist.
        self.head_yaw = 0.0;
        self.head_pitch = neck_look + self.neck_fire_fold + self.neck_hit_fold;
    }

    /// Blaster direction after drive + hip + right shoulder. `None` when not present-armed.
    pub fn weapon_line(&self) -> Option<Vec3> {
        if !self.presents_armed() {
            return None;
        }
        Some(self.weapon_line_dir())
    }

    fn weapon_line_dir(&self) -> Vec3 {
        let fold = self.hip_fire_fold
            + self.hip_hit_fold
            + self.shoulder_fire_fold
            + self.shoulder_hit_fold
            + self.shoulder_sway_fold;
        let twist = self.shoulder_fire_twist + self.shoulder_hit_twist + self.shoulder_sway_twist;
        let yaw = self.ocular_yaw + twist;
        let pitch = (self.ocular_pitch + fold).clamp(-OCULAR_ELEV_CAP_RAD, OCULAR_ELEV_CAP_RAD);
        dir_yaw_pitch(yaw, pitch)
    }

    /// Look origin along weapon line at reticle depth.
    pub fn reticle_world(&self, look_origin: Vec3) -> Option<Vec3> {
        let dir = self.weapon_line()?;
        if dir.length_squared() < 1e-12 {
            return None;
        }
        Some(look_origin + dir * RETICLE_DEPTH_M)
    }

    /// Effective fire residual fall time (s). Slower while fire continues.
    pub fn fire_fall_eff_s(&self, fire_continues: bool) -> f32 {
        let base = self.fire_fall_s.max(1e-4);
        if fire_continues {
            base * FIRE_CONTINUE_FALL_MULT
        } else {
            base
        }
    }

    pub fn apply_fire_impulse(&mut self, def: &WeaponDef, yaw_sign: f32) {
        if !self.alive {
            return;
        }
        let imp = def.fire_impulse;
        let sign = if yaw_sign >= 0.0 { 1.0 } else { -1.0 };
        let fold = imp.pitch_deg.to_radians();
        let twist = imp.yaw_deg.to_radians() * sign;
        let (hip, shoulder, neck) = residual_fold_split(fold);
        self.hip_fire_fold += hip;
        self.shoulder_fire_fold += shoulder;
        self.neck_fire_fold += neck;
        self.shoulder_fire_twist += twist;
        self.grip_bore_m += imp.back_m;
        self.fire_fall_s = imp.fall_s.max(1e-4);
        self.compose_joints();
    }

    /// Hit impulse from applied impact damage. Zero / negative is a no-op.
    pub fn apply_hit_impulse(&mut self, damage: f32, yaw_sign: f32) {
        if !self.alive || damage <= 0.0 {
            return;
        }
        let sign = if yaw_sign >= 0.0 { 1.0 } else { -1.0 };
        let fold = (damage * HIT_FOLD_DEG_PER_DMG)
            .min(HIT_FOLD_CAP_DEG)
            .to_radians();
        let twist = (damage * HIT_TWIST_DEG_PER_DMG)
            .min(HIT_TWIST_CAP_DEG)
            .to_radians()
            * sign;
        let (hip, shoulder, neck) = residual_fold_split(fold);
        let (hip_c, shoulder_c, neck_c) = residual_fold_split(HIT_FOLD_CAP_DEG.to_radians());
        self.hip_hit_fold = (self.hip_hit_fold + hip).clamp(-hip_c.abs(), hip_c.abs());
        self.shoulder_hit_fold =
            (self.shoulder_hit_fold + shoulder).clamp(-shoulder_c.abs(), shoulder_c.abs());
        self.neck_hit_fold = (self.neck_hit_fold + neck).clamp(-neck_c.abs(), neck_c.abs());
        self.shoulder_hit_twist = (self.shoulder_hit_twist + twist).clamp(
            -HIT_TWIST_CAP_DEG.to_radians(),
            HIT_TWIST_CAP_DEG.to_radians(),
        );
        self.hit_fall_s = HIT_FALL_S;
        self.compose_joints();
    }

    pub fn set_shoulder_sway(&mut self, fold: f32, twist: f32) {
        self.shoulder_sway_fold = fold;
        self.shoulder_sway_twist = twist;
        self.compose_joints();
    }

    /// Clear fire residual, sway, and grip bore. Hit residual stays.
    pub fn clear_fire_residual(&mut self) {
        self.hip_fire_fold = 0.0;
        self.shoulder_fire_fold = 0.0;
        self.shoulder_fire_twist = 0.0;
        self.shoulder_sway_fold = 0.0;
        self.shoulder_sway_twist = 0.0;
        self.neck_fire_fold = 0.0;
        self.grip_bore_m = 0.0;
        self.compose_joints();
    }

    /// Clear fire and hit residual (death / full reset).
    pub fn clear_joint_residual(&mut self) {
        self.clear_fire_residual();
        self.hip_hit_fold = 0.0;
        self.shoulder_hit_fold = 0.0;
        self.shoulder_hit_twist = 0.0;
        self.neck_hit_fold = 0.0;
        self.compose_joints();
    }

    /// Copy fire residual fields onto another figure (remote present).
    pub fn copy_fire_residual_to(&self, dst: &mut Self) {
        dst.hip_fire_fold = self.hip_fire_fold;
        dst.shoulder_fire_fold = self.shoulder_fire_fold;
        dst.shoulder_fire_twist = self.shoulder_fire_twist;
        dst.neck_fire_fold = self.neck_fire_fold;
        dst.grip_bore_m = self.grip_bore_m;
        dst.fire_fall_s = self.fire_fall_s;
        dst.compose_joints();
    }

    /// True when fire residual or grip bore is still non-zero.
    pub fn has_fire_residual(&self) -> bool {
        self.hip_fire_fold.abs()
            + self.shoulder_fire_fold.abs()
            + self.shoulder_fire_twist.abs()
            + self.neck_fire_fold.abs()
            + self.grip_bore_m.abs()
            > 1e-5
    }

    /// Fall residual. Fire fall slows while `fire_continues`.
    pub fn tick_joint_residual(&mut self, dt: f32, fire_continues: bool) {
        let dt = dt.max(0.0);
        if !self.alive {
            self.clear_joint_residual();
            return;
        }

        let fire_s = self.fire_fall_eff_s(fire_continues);
        self.hip_fire_fold = fall_toward_zero(self.hip_fire_fold, dt, fire_s);
        self.shoulder_fire_fold = fall_toward_zero(self.shoulder_fire_fold, dt, fire_s);
        self.shoulder_fire_twist = fall_toward_zero(self.shoulder_fire_twist, dt, fire_s);
        self.neck_fire_fold = fall_toward_zero(self.neck_fire_fold, dt, fire_s);
        self.grip_bore_m = {
            let v = fall_toward_zero(self.grip_bore_m, dt, fire_s);
            if v.abs() < 1e-6 {
                0.0
            } else {
                v
            }
        };

        let hit_s = self.hit_fall_s.max(1e-4);
        self.hip_hit_fold = fall_toward_zero(self.hip_hit_fold, dt, hit_s);
        self.shoulder_hit_fold = fall_toward_zero(self.shoulder_hit_fold, dt, hit_s);
        self.shoulder_hit_twist = fall_toward_zero(self.shoulder_hit_twist, dt, hit_s);
        self.neck_hit_fold = fall_toward_zero(self.neck_hit_fold, dt, hit_s);

        self.compose_joints();
    }
}

fn dir_yaw_pitch(yaw: f32, pitch: f32) -> Vec3 {
    let cp = pitch.cos();
    Vec3::new(yaw.sin() * cp, pitch.sin(), yaw.cos() * cp)
}

pub(crate) fn elevation_targets(ocular_pitch: f32) -> (f32, f32) {
    let t = (ocular_pitch / OCULAR_ELEV_CAP_RAD).clamp(-1.0, 1.0);
    if t >= 0.0 {
        (t * TORSO_PITCH_OUTWARD_RAD, t * SHOULDER_PITCH_OUTWARD_RAD)
    } else {
        (t * TORSO_PITCH_INWARD_RAD, t * SHOULDER_PITCH_INWARD_RAD)
    }
}

/// Look-elevation fold weights (hip, shoulder, neck). Sum to 1.
fn residual_fold_shares(sign: f32) -> (f32, f32, f32) {
    let (hip_w, shoulder_w) = if sign >= 0.0 {
        (TORSO_PITCH_OUTWARD_RAD, SHOULDER_PITCH_OUTWARD_RAD)
    } else {
        (TORSO_PITCH_INWARD_RAD, SHOULDER_PITCH_INWARD_RAD)
    };
    let neck_w = HEAD_PITCH_BUDGET_RAD;
    let sum = hip_w + shoulder_w + neck_w;
    (hip_w / sum, shoulder_w / sum, neck_w / sum)
}

pub(crate) fn residual_fold_split(total: f32) -> (f32, f32, f32) {
    if total.abs() < 1e-12 {
        return (0.0, 0.0, 0.0);
    }
    let (sh, ss, sn) = residual_fold_shares(total);
    (total * sh, total * ss, total * sn)
}
