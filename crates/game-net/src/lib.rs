//! Directional postcard multiplayer wire.

use serde::{Deserialize, Serialize};

pub const TICK_HZ: u32 = 180;

pub const TICK_DURATION_SECS: f64 = 1.0 / TICK_HZ as f64;

pub const PROTOCOL_VERSION: u16 = 3;

pub type PlayerId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NetVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl NetVec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetLocomotion {
    Stand,
    Walk,
    Sprint,
    Stopping,
    Air,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetActiveWeapon {
    Primary,
    Secondary,
}

/// Enough fields to rebuild present pose on a peer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriveView {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientToServer {
    Hello { protocol: u16 },
    ClockProbe { t1: f64 },
    DriveSample { tick: u64, drive: DriveView },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerToClient {
    Welcome {
        protocol: u16,
        player_id: PlayerId,
        tick: u64,
        server_time_secs: f64,
    },
    Reject {
        reason: String,
    },
    ClockReply {
        t1: f64,
        t2: f64,
        t3: f64,
        tick: u64,
    },
    PeerJoined {
        tick: u64,
        id: PlayerId,
    },
    PeerLeft {
        tick: u64,
        id: PlayerId,
    },
    PeerDrive {
        tick: u64,
        id: PlayerId,
        drive: DriveView,
    },
}

pub fn encode_c2s(msg: &ClientToServer) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(msg)
}

pub fn decode_c2s(buf: &[u8]) -> Result<ClientToServer, postcard::Error> {
    postcard::from_bytes(buf)
}

pub fn encode_s2c(msg: &ServerToClient) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(msg)
}

pub fn decode_s2c(buf: &[u8]) -> Result<ServerToClient, postcard::Error> {
    postcard::from_bytes(buf)
}

/// `u32` LE length + postcard body (reliable stream after Welcome).
pub fn encode_s2c_frame(msg: &ServerToClient) -> Result<Vec<u8>, postcard::Error> {
    let body = encode_s2c(msg)?;
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// Decode complete frames; leaves a partial tail in `buf`.
pub fn drain_s2c_frames(buf: &mut Vec<u8>) -> Vec<ServerToClient> {
    let mut out = Vec::new();
    loop {
        if buf.len() < 4 {
            break;
        }
        let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if buf.len() < 4 + len {
            break;
        }
        let body: Vec<u8> = buf.drain(..4 + len).skip(4).collect();
        if let Ok(msg) = decode_s2c(&body) {
            out.push(msg);
        }
    }
    out
}

/// `server ≈ local + offset` → floor to tick.
pub fn estimated_tick(local_secs: f64, offset_secs: f64) -> u64 {
    let server = local_secs + offset_secs;
    if server <= 0.0 {
        0
    } else {
        (server * TICK_HZ as f64).floor() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_drive() -> DriveView {
        DriveView {
            position: NetVec3::new(1.0, 0.0, 2.0),
            ocular_yaw: 0.5,
            ocular_pitch: -0.1,
            character: b'a',
            primary: Some(b'p'),
            secondary: Some(b'b'),
            active: NetActiveWeapon::Primary,
            locomotion: NetLocomotion::Walk,
            walk_phase: 0.25,
            velocity_y: 0.0,
        }
    }

    #[test]
    fn hello_roundtrip() {
        let msg = ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
        };
        let b = encode_c2s(&msg).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), msg);
    }

    #[test]
    fn welcome_roundtrip() {
        let msg = ServerToClient::Welcome {
            protocol: 2,
            player_id: 7,
            tick: 42,
            server_time_secs: 1.5,
        };
        let b = encode_s2c(&msg).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), msg);
    }

    #[test]
    fn reject_roundtrip() {
        let msg = ServerToClient::Reject {
            reason: "protocol mismatch".into(),
        };
        let b = encode_s2c(&msg).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), msg);
    }

    #[test]
    fn clock_probe_reply_roundtrip() {
        let probe = ClientToServer::ClockProbe { t1: 0.25 };
        let b = encode_c2s(&probe).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), probe);

        let reply = ServerToClient::ClockReply {
            t1: 0.1,
            t2: 0.2,
            t3: 0.21,
            tick: 7,
        };
        let b = encode_s2c(&reply).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), reply);
    }

    #[test]
    fn drive_sample_roundtrip() {
        let msg = ClientToServer::DriveSample {
            tick: 99,
            drive: sample_drive(),
        };
        let b = encode_c2s(&msg).unwrap();
        assert_eq!(decode_c2s(&b).unwrap(), msg);
    }

    #[test]
    fn peer_drive_roundtrip() {
        let msg = ServerToClient::PeerDrive {
            tick: 100,
            id: 3,
            drive: sample_drive(),
        };
        let b = encode_s2c(&msg).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), msg);
    }

    #[test]
    fn peer_joined_left_roundtrip() {
        let joined = ServerToClient::PeerJoined { tick: 10, id: 2 };
        let b = encode_s2c(&joined).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), joined);

        let left = ServerToClient::PeerLeft { tick: 11, id: 2 };
        let b = encode_s2c(&left).unwrap();
        assert_eq!(decode_s2c(&b).unwrap(), left);
    }

    #[test]
    fn estimated_tick_basic() {
        assert_eq!(estimated_tick(1.0, 0.0), 180);
    }

    #[test]
    fn directional_roots_do_not_cross_decode() {
        let c2s = encode_c2s(&ClientToServer::Hello { protocol: 1 }).unwrap();
        assert!(decode_s2c(&c2s).is_err());
        let s2c = encode_s2c(&ServerToClient::Reject { reason: "x".into() }).unwrap();
        assert!(decode_c2s(&s2c).is_err());
    }

    #[test]
    fn framed_s2c_roundtrip_batch() {
        let a = ServerToClient::PeerJoined { tick: 1, id: 2 };
        let b = ServerToClient::PeerLeft { tick: 3, id: 2 };
        let mut buf = encode_s2c_frame(&a).unwrap();
        buf.extend(encode_s2c_frame(&b).unwrap());
        // partial tail
        buf.extend_from_slice(&[3, 0, 0, 0, 9]);
        let msgs = drain_s2c_frames(&mut buf);
        assert_eq!(msgs, vec![a, b]);
        assert_eq!(buf, vec![3, 0, 0, 0, 9]);
    }
}
