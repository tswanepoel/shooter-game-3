//! Inbound S2C queue (decode + optional discard seam for 025).

use std::collections::VecDeque;

use game_net::{decode_s2c, DecodeError, ServerToClient, CONTENT_REV};

pub struct InboundQueue {
    pending: VecDeque<ServerToClient>,
    pub decoded: u32,
    pub discarded: u32,
}

impl InboundQueue {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            decoded: 0,
            discarded: 0,
        }
    }

    /// Decode binary frame. Malformed bytes are dropped (counted).
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        match decode_s2c(bytes) {
            Ok(msg) => {
                if should_discard_content(&msg) {
                    self.discarded = self.discarded.wrapping_add(1);
                    return;
                }
                self.decoded = self.decoded.wrapping_add(1);
                self.pending.push_back(msg);
            }
            Err(DecodeError::Postcard(_)) => {
                self.discarded = self.discarded.wrapping_add(1);
            }
        }
    }

    pub fn pop(&mut self) -> Option<ServerToClient> {
        self.pending.pop_front()
    }
}

impl Default for InboundQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn should_discard_content(msg: &ServerToClient) -> bool {
    match msg {
        ServerToClient::Welcome(w) => w.content_rev != CONTENT_REV,
        // Other messages inherit session after Welcome; silent content path is Hello/Welcome.
        _ => false,
    }
}
