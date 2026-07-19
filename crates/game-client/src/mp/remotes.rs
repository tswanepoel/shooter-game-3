//! Remote peer pose buffers, present clock, and sampling (024 / 027 / 028).

use std::collections::{HashMap, VecDeque};
use std::f32::consts::{PI, TAU};

use game_net::{NetPlayerPose, NetVec3, PlayerId, Tick};

/// Draw remotes this far behind the present clock (027). ~3 ticks at 30 Hz.
pub const REMOTE_INTERP_DELAY_SECS: f32 = 0.100;

/// Server tick rate used to map `tick` → seconds (must match server `TICK_HZ`).
const SERVER_TICK_HZ: f32 = 30.0;

/// Samples kept per remote (~1 s at 30 Hz).
const BUFFER_CAP: usize = 32;

/// If estimated clock is this far behind a new snapshot tick, snap (tab stall).
const CLOCK_SNAP_BEHIND_SECS: f32 = 0.5;

/// Character + loadout letters that decide which meshes a remote body needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RemoteKitKey {
    pub character: u8,
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
}

impl RemoteKitKey {
    pub fn from_pose(pose: &NetPlayerPose) -> Self {
        Self {
            character: pose.character,
            primary: pose.primary,
            secondary: pose.secondary,
        }
    }
}

#[derive(Debug, Clone)]
struct Sample {
    tick: Tick,
    pose: NetPlayerPose,
}

#[derive(Debug, Default)]
struct RemoteTrack {
    samples: VecDeque<Sample>,
}

impl RemoteTrack {
    fn push(&mut self, tick: Tick, pose: NetPlayerPose) {
        if let Some(back) = self.samples.back_mut() {
            if back.tick == tick {
                back.pose = pose;
                return;
            }
            if tick < back.tick {
                // Out-of-order; ignore older.
                return;
            }
        }
        self.samples.push_back(Sample { tick, pose });
        while self.samples.len() > BUFFER_CAP {
            self.samples.pop_front();
        }
    }

    fn latest(&self) -> Option<&NetPlayerPose> {
        self.samples.back().map(|s| &s.pose)
    }

    /// Sample at absolute time `t_secs` (tick / SERVER_TICK_HZ).
    fn sample_at(&self, t_secs: f32) -> Option<NetPlayerPose> {
        if self.samples.is_empty() {
            return None;
        }
        if self.samples.len() == 1 {
            return Some(self.samples[0].pose.clone());
        }

        let first_t = tick_to_secs(self.samples[0].tick);
        if t_secs <= first_t {
            return Some(self.samples[0].pose.clone());
        }

        let last = self.samples.back().unwrap();
        let last_t = tick_to_secs(last.tick);
        if t_secs >= last_t {
            // Underrun: hold last (no extrapolate).
            return Some(last.pose.clone());
        }

        for i in 0..self.samples.len() - 1 {
            let a = &self.samples[i];
            let b = &self.samples[i + 1];
            let ta = tick_to_secs(a.tick);
            let tb = tick_to_secs(b.tick);
            if t_secs >= ta && t_secs <= tb {
                let denom = (tb - ta).max(1e-6);
                let u = ((t_secs - ta) / denom).clamp(0.0, 1.0);
                return Some(lerp_pose(&a.pose, &b.pose, u));
            }
        }

        Some(last.pose.clone())
    }
}

/// Buffered remote poses; present via delayed interpolation on a frame clock (028).
pub struct RemoteTable {
    tracks: HashMap<PlayerId, RemoteTrack>,
    last_tick: Tick,
    /// Estimated server time (seconds). Advances every frame; pulled up on Snapshot.
    server_clock: f32,
    /// True after the first Snapshot so we do not free-run from 0.
    clock_live: bool,
}

impl RemoteTable {
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
            last_tick: 0,
            server_clock: 0.0,
            clock_live: false,
        }
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.last_tick = 0;
        self.server_clock = 0.0;
        self.clock_live = false;
    }

    /// Append Snapshot `others` at server `tick`; pull present clock up.
    pub fn apply_snapshot_others(&mut self, tick: Tick, others: Vec<NetPlayerPose>) {
        self.last_tick = tick;
        let target = tick_to_secs(tick);
        if !self.clock_live {
            self.server_clock = target;
            self.clock_live = true;
        } else if target - self.server_clock > CLOCK_SNAP_BEHIND_SECS {
            // Large stall (tab background, etc.): hard snap.
            self.server_clock = target;
        } else {
            self.server_clock = self.server_clock.max(target);
        }

        let mut live = HashMap::with_capacity(others.len());
        for pose in others {
            live.insert(pose.id, pose);
        }
        self.tracks.retain(|id, _| live.contains_key(id));
        for (id, pose) in live {
            self.tracks.entry(id).or_default().push(tick, pose);
        }
    }

    /// Advance the estimated server clock by one render frame (028).
    pub fn advance(&mut self, dt: f32) {
        if !self.clock_live {
            return;
        }
        self.server_clock += dt.max(0.0);
    }

    pub fn remove(&mut self, id: PlayerId) {
        self.tracks.remove(&id);
    }

    pub fn count(&self) -> usize {
        self.tracks.len()
    }

    pub fn ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.tracks.keys().copied()
    }

    /// Newest sample per peer (kit load / identity).
    pub fn latest_poses(&self) -> Vec<NetPlayerPose> {
        self.tracks
            .values()
            .filter_map(|t| t.latest().cloned())
            .collect()
    }

    /// Interpolated poses at `server_clock − delay` (advances every frame after [`advance`]).
    pub fn present_poses(&self) -> Vec<NetPlayerPose> {
        if !self.clock_live {
            return Vec::new();
        }
        let present_t = self.server_clock - REMOTE_INTERP_DELAY_SECS;
        self.tracks
            .values()
            .filter_map(|t| t.sample_at(present_t))
            .collect()
    }
}

impl Default for RemoteTable {
    fn default() -> Self {
        Self::new()
    }
}

fn tick_to_secs(tick: Tick) -> f32 {
    tick as f32 / SERVER_TICK_HZ
}

fn lerp_pose(a: &NetPlayerPose, b: &NetPlayerPose, u: f32) -> NetPlayerPose {
    // Discrete fields from the newer sample.
    NetPlayerPose {
        id: b.id,
        position: NetVec3::new(
            lerp(a.position.x, b.position.x, u),
            lerp(a.position.y, b.position.y, u),
            lerp(a.position.z, b.position.z, u),
        ),
        ocular_yaw: lerp_angle(a.ocular_yaw, b.ocular_yaw, u),
        ocular_pitch: lerp(a.ocular_pitch, b.ocular_pitch, u),
        character: b.character,
        primary: b.primary,
        secondary: b.secondary,
        active: b.active,
        locomotion: b.locomotion,
        walk_phase: lerp_phase(a.walk_phase, b.walk_phase, u),
        velocity_y: lerp(a.velocity_y, b.velocity_y, u),
    }
}

fn lerp(a: f32, b: f32, u: f32) -> f32 {
    a + (b - a) * u
}

fn lerp_angle(a: f32, b: f32, u: f32) -> f32 {
    let mut d = b - a;
    if d > PI {
        d -= TAU;
    } else if d < -PI {
        d += TAU;
    }
    a + d * u
}

/// Phase is in [0, 1); take the short way around the cycle when blending.
fn lerp_phase(a: f32, b: f32, u: f32) -> f32 {
    let mut d = b - a;
    if d > 0.5 {
        d -= 1.0;
    } else if d < -0.5 {
        d += 1.0;
    }
    let mut p = a + d * u;
    if p < 0.0 {
        p += 1.0;
    } else if p >= 1.0 {
        p -= 1.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_net::{NetActiveWeapon, NetLocomotion};

    fn pose(id: PlayerId, x: f32, z: f32) -> NetPlayerPose {
        NetPlayerPose {
            id,
            position: NetVec3::new(x, 0.0, z),
            ocular_yaw: 0.0,
            ocular_pitch: 0.0,
            character: b'a',
            primary: Some(b'p'),
            secondary: None,
            active: NetActiveWeapon::Primary,
            locomotion: NetLocomotion::Walk,
            walk_phase: 0.0,
            velocity_y: 0.0,
        }
    }

    #[test]
    fn holds_last_when_present_past_newest() {
        let mut track = RemoteTrack::default();
        track.push(10, pose(1, 0.0, 0.0));
        track.push(11, pose(1, 0.0, 1.0));
        let p = track.sample_at(tick_to_secs(100)).unwrap();
        assert!((p.position.z - 1.0).abs() < 1e-5);
    }

    #[test]
    fn lerps_between_ticks() {
        let mut track = RemoteTrack::default();
        track.push(10, pose(1, 0.0, 0.0));
        track.push(12, pose(1, 0.0, 2.0));
        let mid = (tick_to_secs(10) + tick_to_secs(12)) * 0.5;
        let p = track.sample_at(mid).unwrap();
        assert!((p.position.z - 1.0).abs() < 1e-4);
    }

    #[test]
    fn snapshot_drops_missing_peers() {
        let mut table = RemoteTable::new();
        table.apply_snapshot_others(1, vec![pose(1, 0.0, 0.0), pose(2, 1.0, 0.0)]);
        assert_eq!(table.count(), 2);
        table.apply_snapshot_others(2, vec![pose(1, 0.0, 1.0)]);
        assert_eq!(table.count(), 1);
        assert_eq!(table.latest_poses()[0].id, 1);
    }

    #[test]
    fn present_uses_delay_from_clock() {
        let mut table = RemoteTable::new();
        table.apply_snapshot_others(100, vec![pose(1, 0.0, 0.0)]);
        table.apply_snapshot_others(110, vec![pose(1, 0.0, 10.0)]);
        // clock at 110/30; present = clock - 0.1 → ~70% along 100→110 segment.
        let poses = table.present_poses();
        assert_eq!(poses.len(), 1);
        let z = poses[0].position.z;
        assert!(z > 5.0 && z < 10.0, "z={z}");
    }

    #[test]
    fn advance_moves_present_between_snapshots() {
        let mut table = RemoteTable::new();
        table.apply_snapshot_others(100, vec![pose(1, 0.0, 0.0)]);
        table.apply_snapshot_others(110, vec![pose(1, 0.0, 10.0)]);
        let z0 = table.present_poses()[0].position.z;
        // Advance without a new snapshot; present time should crawl forward.
        table.advance(1.0 / 60.0);
        let z1 = table.present_poses()[0].position.z;
        assert!(
            z1 > z0,
            "frame advance should move interp forward: z0={z0} z1={z1}"
        );
    }

    #[test]
    fn clear_resets_clock() {
        let mut table = RemoteTable::new();
        table.apply_snapshot_others(50, vec![pose(1, 0.0, 0.0)]);
        assert!(table.clock_live);
        table.clear();
        assert!(!table.clock_live);
        assert!(table.present_poses().is_empty());
    }

    #[test]
    fn large_stall_snaps_clock() {
        let mut table = RemoteTable::new();
        table.apply_snapshot_others(10, vec![pose(1, 0.0, 0.0)]);
        // Simulate free-run falling behind: force clock old, then big tick jump.
        table.server_clock = tick_to_secs(10);
        table.apply_snapshot_others(100, vec![pose(1, 0.0, 5.0)]);
        assert!((table.server_clock - tick_to_secs(100)).abs() < 1e-5);
    }
}
