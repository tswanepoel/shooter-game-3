//! Shared wire scalars (not sim types).

/// Server-assigned player identity.
pub type PlayerId = u32;

/// Server simulation tick index.
pub type Tick = u32;

/// Client outbound sequence number.
pub type Seq = u32;

/// Server-recycled session binding key (echoed on Input).
pub type SessionKey = u64;

/// Cook / content stamp. Mismatch → silent discard at the boundary.
pub type ContentRev = u32;

/// Wire protocol version. Mismatch ends join.
pub type Protocol = u16;

/// Position on the wire (metres, Y-up).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NetVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl NetVec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}
