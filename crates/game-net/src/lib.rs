//! Directional postcard multiplayer wire.

use serde::{Deserialize, Serialize};

pub const TICK_HZ: u32 = 30;

pub const TICK_DURATION_SECS: f64 = 1.0 / TICK_HZ as f64;

pub const PROTOCOL_VERSION: u16 = 1;

pub type PlayerId = u32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientToServer {
    Hello { protocol: u16 },
    ClockProbe { t1: f64 },
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
            protocol: 1,
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
    fn estimated_tick_basic() {
        assert_eq!(estimated_tick(1.0, 0.0), 30);
    }

    #[test]
    fn directional_roots_do_not_cross_decode() {
        let c2s = encode_c2s(&ClientToServer::Hello { protocol: 1 }).unwrap();
        assert!(decode_s2c(&c2s).is_err());
        let s2c = encode_s2c(&ServerToClient::Reject { reason: "x".into() }).unwrap();
        assert!(decode_c2s(&s2c).is_err());
    }
}
