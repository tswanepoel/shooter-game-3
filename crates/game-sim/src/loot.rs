//! Ammo drops (059) and blaster drops (067). Corpse is separate.

use glam::Vec3;

use crate::{
    clamp_blaster_mag, max_dump_rounds_for, AmmoKind, AMMO_DROP_LIFETIME_S,
    BLASTER_DROP_LIFETIME_S, BLASTER_LOOK_DOT_MIN, DEATH_BLASTER_BACK_M, DEATH_BLASTER_RIGHT_M,
    LOOT_TAKE_RADIUS_M, SWAP_BLASTER_FORWARD_M,
};

/// Invisible ammo drop pinned at a world position (059).
#[derive(Debug, Clone, PartialEq)]
pub struct AmmoDrop {
    pub id: u64,
    pub position: Vec3,
    pub kind: AmmoKind,
    pub rounds: u16,
    pub age_s: f32,
}

impl AmmoDrop {
    pub fn new(id: u64, position: Vec3, kind: AmmoKind, rounds: u16) -> Self {
        Self {
            id,
            position,
            kind,
            rounds,
            age_s: 0.0,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.age_s += dt.max(0.0);
    }

    pub fn expired(&self) -> bool {
        self.age_s >= AMMO_DROP_LIFETIME_S || self.rounds == 0
    }

    /// How many rounds a taker with `reserve_room` may take this step.
    pub fn take_amount(&self, reserve_room: u16) -> u16 {
        self.rounds.min(reserve_room)
    }

    /// Remove up to `n` rounds; returns how many removed.
    pub fn take_rounds(&mut self, n: u16) -> u16 {
        let got = self.rounds.min(n);
        self.rounds -= got;
        got
    }
}

/// Visible floor blaster: letter + magazine (067).
#[derive(Debug, Clone, PartialEq)]
pub struct BlasterDrop {
    pub id: u64,
    pub position: Vec3,
    pub letter: u8,
    pub mag: u16,
    pub age_s: f32,
}

impl BlasterDrop {
    pub fn new(id: u64, position: Vec3, letter: u8, mag: u16) -> Self {
        Self {
            id,
            position,
            letter,
            mag,
            age_s: 0.0,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        self.age_s += dt.max(0.0);
    }

    pub fn expired(&self) -> bool {
        self.age_s >= BLASTER_DROP_LIFETIME_S
    }
}

/// Clamp a victim-reported ammo dump to a plausible payload (059 / 067 reserve-only).
pub fn clamp_dump_rounds(kind: AmmoKind, rounds: u16) -> u16 {
    rounds.min(max_dump_rounds_for(kind))
}

/// Clamp a blaster dump magazine (067).
pub fn clamp_blaster_dump(letter: u8, mag: u16) -> Option<(u8, u16)> {
    let mag = clamp_blaster_mag(letter, mag)?;
    Some((letter, mag))
}

/// True when `pos` is within take radius of the drop.
pub fn in_take_radius(drop_pos: Vec3, pos: Vec3) -> bool {
    drop_pos.distance(pos) <= LOOT_TAKE_RADIUS_M
}

/// Death dump pose: right of the body (Kenney right-hand hold) and slightly behind
/// (die falls backwards), so the mesh is not inside the corpse torso (067).
pub fn death_blaster_drop_position(feet: Vec3, facing: f32) -> Vec3 {
    let forward = Vec3::new(facing.sin(), 0.0, facing.cos());
    let right = Vec3::new(-forward.z, 0.0, forward.x);
    feet + right * DEATH_BLASTER_RIGHT_M - forward * DEATH_BLASTER_BACK_M
}

/// Displaced blaster lands in front of the figure, not under the soles (067).
pub fn swap_blaster_drop_position(feet: Vec3, look_yaw: f32) -> Vec3 {
    let forward = Vec3::new(look_yaw.sin(), 0.0, look_yaw.cos());
    feet + forward * SWAP_BLASTER_FORWARD_M
}

/// Approximate look origin for loot aim checks (feet + standing eye height).
pub fn loot_look_origin(feet: Vec3) -> Vec3 {
    feet + Vec3::new(0.0, 1.52, 0.0)
}

/// Drive look direction from yaw / pitch (unit).
pub fn look_forward_dir(look_yaw: f32, look_pitch: f32) -> Vec3 {
    let cp = look_pitch.cos();
    Vec3::new(look_yaw.sin() * cp, look_pitch.sin(), look_yaw.cos() * cp)
}

/// True when look points near enough at the drop (radius still required separately).
pub fn looking_at_blaster(eye: Vec3, look_fwd: Vec3, drop_pos: Vec3) -> bool {
    let to = drop_pos - eye;
    let dist_sq = to.length_squared();
    if dist_sq < 1e-6 {
        return true;
    }
    let dir = to / dist_sq.sqrt();
    let fwd = look_fwd.normalize_or_zero();
    if fwd.length_squared() < 1e-6 {
        return false;
    }
    dir.dot(fwd) >= BLASTER_LOOK_DOT_MIN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_partial_leaves_rest() {
        let mut d = AmmoDrop::new(1, Vec3::ZERO, AmmoKind::LightFoam, 10);
        assert_eq!(d.take_rounds(4), 4);
        assert_eq!(d.rounds, 6);
        assert!(!d.expired());
        assert_eq!(d.take_rounds(100), 6);
        assert!(d.expired());
    }

    #[test]
    fn clamp_dump_respects_cap() {
        assert_eq!(
            clamp_dump_rounds(AmmoKind::Grenade, 999),
            max_dump_rounds_for(AmmoKind::Grenade)
        );
        assert_eq!(clamp_dump_rounds(AmmoKind::Grenade, 2), 2);
    }

    #[test]
    fn clamp_blaster_mag_caps() {
        let (letter, mag) = clamp_blaster_dump(b'b', 999).expect("pistol");
        assert_eq!(letter, b'b');
        assert_eq!(mag, 1);
        assert!(clamp_blaster_dump(b'z', 1).is_none());
    }

    #[test]
    fn blaster_drop_expires_on_timer() {
        let mut d = BlasterDrop::new(1, Vec3::ZERO, b'b', 0);
        assert!(!d.expired());
        d.age_s = BLASTER_DROP_LIFETIME_S;
        assert!(d.expired());
    }

    #[test]
    fn death_drop_offset_is_right_and_back() {
        let p = death_blaster_drop_position(Vec3::ZERO, 0.0);
        // Facing +Z: screen-right is −X (look_right_xz), back is −Z.
        assert!(p.x < -0.4);
        assert!(p.z < -0.2);
    }

    #[test]
    fn swap_drop_is_in_front() {
        let p = swap_blaster_drop_position(Vec3::ZERO, 0.0);
        assert!(p.z > 0.5);
        assert!(p.x.abs() < 1e-5);
    }

    #[test]
    fn looking_at_rejects_sky_gaze() {
        let eye = loot_look_origin(Vec3::ZERO);
        let drop = Vec3::new(0.0, 0.0, 1.0);
        let sky = look_forward_dir(0.0, 1.4);
        assert!(!looking_at_blaster(eye, sky, drop));
        let ahead = look_forward_dir(0.0, -0.9);
        assert!(looking_at_blaster(eye, ahead, drop));
    }
}
