//! Outbound C2S queue (encode seam for later delay tools).

use std::collections::VecDeque;

use game_net::{encode_c2s, ClientToServer, EncodeError};

pub struct OutboundQueue {
    pending: VecDeque<ClientToServer>,
}

impl OutboundQueue {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    pub fn push(&mut self, msg: ClientToServer) {
        self.pending.push_back(msg);
    }

    /// Drain and encode all pending messages.
    pub fn drain_encoded(&mut self) -> Result<Vec<Vec<u8>>, EncodeError> {
        let mut out = Vec::with_capacity(self.pending.len());
        while let Some(msg) = self.pending.pop_front() {
            out.push(encode_c2s(&msg)?);
        }
        Ok(out)
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn pop_discard(&mut self) -> bool {
        self.pending.pop_front().is_some()
    }
}

impl Default for OutboundQueue {
    fn default() -> Self {
        Self::new()
    }
}
