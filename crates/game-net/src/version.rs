//! Protocol and content revision constants.

use crate::types::{ContentRev, Protocol};

/// Current wire protocol. Bump when `ClientToServer` / `ServerToClient` layout changes
/// or when `TICK_HZ` changes (tick → seconds must match on both ends).
pub const PROTOCOL_VERSION: Protocol = 4;

/// Content stamp carried on Hello/Welcome. Bump when cooked kits change identity rules.
pub const CONTENT_REV: ContentRev = 1;

/// Fixed server simulation rate (Hz). Client maps `tick` → seconds with the same value.
pub const TICK_HZ: u32 = 128;

/// One authority step in seconds (`1 / TICK_HZ`). Land quantize + post-arrival buffer (032).
pub const TICK_DURATION_SECS: f32 = 1.0 / TICK_HZ as f32;
