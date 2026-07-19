//! Protocol and content revision constants.

use crate::types::{ContentRev, Protocol};

/// Current wire protocol. Bump when `ClientToServer` / `ServerToClient` layout changes.
pub const PROTOCOL_VERSION: Protocol = 2;

/// Content stamp carried on Hello/Welcome. Bump when cooked kits change identity rules.
pub const CONTENT_REV: ContentRev = 1;
