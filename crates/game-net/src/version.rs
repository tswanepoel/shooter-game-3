//! Protocol and content revision constants.

use crate::types::{ContentRev, Protocol};

/// Current wire protocol. Bump when `ClientToServer` / `ServerToClient` layout changes
/// or when `TICK_HZ` changes (tick → seconds must match on both ends).
pub const PROTOCOL_VERSION: Protocol = 3;

/// Content stamp carried on Hello/Welcome. Bump when cooked kits change identity rules.
pub const CONTENT_REV: ContentRev = 1;

/// Fixed server simulation rate (Hz). Client maps `tick` → seconds with the same value.
pub const TICK_HZ: u32 = 128;
