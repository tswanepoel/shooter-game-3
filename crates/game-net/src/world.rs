//! World slice: snapshots and peer lifecycle.

use crate::types::NetVec3;
use crate::types::{PlayerId, Seq, SessionKey, Tick};

/// Locomotion mode on the wire (mirrors sim; independent type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NetLocomotion {
    Stand,
    Walk,
    Sprint,
    Stopping,
    Air,
}

/// Active loadout hand on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NetActiveWeapon {
    Primary,
    Secondary,
}

/// Server spawn placement (ground plane + yaw).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NetSpawn {
    pub position: NetVec3,
    pub yaw: f32,
}

/// Authoritative player drive for presentation (local `you` or a remote).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NetPlayerPose {
    pub id: PlayerId,
    pub position: NetVec3,
    pub ocular_yaw: f32,
    pub ocular_pitch: f32,
    pub character: u8,
    pub primary: Option<u8>,
    pub secondary: Option<u8>,
    pub active: NetActiveWeapon,
    pub locomotion: NetLocomotion,
    pub walk_phase: f32,
    pub velocity_y: f32,
}

/// S→C: world sample at a server tick.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub tick: Tick,
    pub key: SessionKey,
    pub issued_tick: Tick,
    /// Last `Input.seq` applied for the recipient into the sim that produced `you`.
    /// `0` before any Input has been applied.
    pub ack_seq: Seq,
    /// Full body-channel land delay for the recipient: `L_i + TICK_DURATION_SECS` (032).
    pub land_delay_secs: f32,
    /// Smoothed uplink `L_i` for the recipient (intent → server recv).
    pub uplink_secs: f32,
    /// Session floor `L_min` (min smoothed uplink among joined peers).
    pub l_min_secs: f32,
    pub you: Option<NetPlayerPose>,
    pub others: Vec<NetPlayerPose>,
}

/// S→C: peer left the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlayerLeft {
    pub id: PlayerId,
}
