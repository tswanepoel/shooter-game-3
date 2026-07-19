//! Multiplayer wire protocol: directed messages and postcard codec.
//!
//! No sockets, no sim apply, no GPU. Client and server map to/from these types
//! at the boundary.

mod codec;
mod movement;
mod session;
mod types;
mod version;
mod world;

pub use codec::{decode_c2s, decode_s2c, encode_c2s, encode_s2c, DecodeError, EncodeError};
pub use movement::Input;
pub use session::{Hello, Reject, RejectReason, Welcome};
pub use types::{ContentRev, NetVec3, PlayerId, Protocol, Seq, SessionKey, Tick};
pub use version::{CONTENT_REV, PROTOCOL_VERSION};
pub use world::{NetActiveWeapon, NetLocomotion, NetPlayerPose, NetSpawn, PlayerLeft, Snapshot};

/// Client → server only.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClientToServer {
    Hello(Hello),
    Input(Input),
}

/// Server → client only.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ServerToClient {
    Welcome(Welcome),
    Reject(Reject),
    Snapshot(Snapshot),
    PlayerLeft(PlayerLeft),
}
