//! Postcard encode/decode for directed messages.

use crate::{ClientToServer, ServerToClient};

/// Encode failure (allocation / postcard).
#[derive(Debug)]
pub struct EncodeError(pub postcard::Error);

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "encode: {}", self.0)
    }
}

impl std::error::Error for EncodeError {}

/// Decode outcome: valid message, or drop without panic.
#[derive(Debug)]
pub enum DecodeError {
    Postcard(postcard::Error),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postcard(e) => write!(f, "decode: {e}"),
        }
    }
}

impl std::error::Error for DecodeError {}

pub fn encode_c2s(msg: &ClientToServer) -> Result<Vec<u8>, EncodeError> {
    postcard::to_allocvec(msg).map_err(EncodeError)
}

pub fn decode_c2s(bytes: &[u8]) -> Result<ClientToServer, DecodeError> {
    postcard::from_bytes(bytes).map_err(DecodeError::Postcard)
}

pub fn encode_s2c(msg: &ServerToClient) -> Result<Vec<u8>, EncodeError> {
    postcard::to_allocvec(msg).map_err(EncodeError)
}

pub fn decode_s2c(bytes: &[u8]) -> Result<ServerToClient, DecodeError> {
    postcard::from_bytes(bytes).map_err(DecodeError::Postcard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::Input;
    use crate::session::{Hello, Welcome};
    use crate::types::NetVec3;
    use crate::version::{CONTENT_REV, PROTOCOL_VERSION};
    use crate::world::{NetSpawn, Snapshot};

    #[test]
    fn roundtrip_hello() {
        let msg = ClientToServer::Hello(Hello {
            protocol: PROTOCOL_VERSION,
            content_rev: CONTENT_REV,
        });
        let bytes = encode_c2s(&msg).unwrap();
        assert_eq!(decode_c2s(&bytes).unwrap(), msg);
    }

    #[test]
    fn roundtrip_welcome_snapshot_input() {
        let welcome = ServerToClient::Welcome(Welcome {
            you: 1,
            tick: 0,
            spawn: NetSpawn {
                position: NetVec3::new(1.0, 0.0, 2.0),
                yaw: 0.5,
            },
            key: 0xDEAD_BEEF,
            issued_tick: 0,
            content_rev: CONTENT_REV,
        });
        let wbytes = encode_s2c(&welcome).unwrap();
        assert_eq!(decode_s2c(&wbytes).unwrap(), welcome);

        let snap = ServerToClient::Snapshot(Snapshot {
            tick: 3,
            key: 0xDEAD_BEEF,
            issued_tick: 0,
            ack_seq: 12,
            you: None,
            others: vec![],
        });
        let sbytes = encode_s2c(&snap).unwrap();
        assert_eq!(decode_s2c(&sbytes).unwrap(), snap);

        let input = ClientToServer::Input(Input {
            seq: 9,
            echo_key: 0xDEAD_BEEF,
            echo_issued_tick: 0,
            wish_forward: 1.0,
            wish_strafe: 0.0,
            look_yaw: 0.1,
            look_pitch: -0.2,
            jump: false,
            sprint_tap: false,
            weapon_cycle: 0,
        });
        let ibytes = encode_c2s(&input).unwrap();
        assert_eq!(decode_c2s(&ibytes).unwrap(), input);
    }

    #[test]
    fn garbage_is_decode_error() {
        assert!(decode_c2s(&[0xff, 0x00, 0x01]).is_err());
    }
}
