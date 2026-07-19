//! Session slice: connect handshake and identity.

use crate::types::{ContentRev, PlayerId, Protocol, SessionKey, Tick};
use crate::world::NetSpawn;

/// C→S: open a join attempt.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Hello {
    pub protocol: Protocol,
    pub content_rev: ContentRev,
}

/// S→C: accepted into the world.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Welcome {
    pub you: PlayerId,
    pub tick: Tick,
    pub spawn: NetSpawn,
    pub key: SessionKey,
    pub issued_tick: Tick,
    pub content_rev: ContentRev,
}

/// S→C: join failed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Reject {
    pub reason: RejectReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RejectReason {
    ProtocolMismatch,
    /// Server full or other accept failure.
    Unavailable,
}
