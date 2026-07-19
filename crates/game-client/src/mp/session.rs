//! Join lifecycle and session key storage (client obeys server values).

use game_net::{PlayerId, SessionKey, Tick, CONTENT_REV};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpPhase {
    /// Local sim only; no server.
    Solo,
    Connecting,
    Joined,
}

/// Multiplayer session state. Key/tick always come from the last S→C.
pub struct MpSession {
    phase: MpPhase,
    pub you: Option<PlayerId>,
    pub server_tick: Tick,
    pub key: SessionKey,
    pub key_issued_tick: Tick,
    pub content_rev: u32,
}

impl MpSession {
    pub fn solo() -> Self {
        Self {
            phase: MpPhase::Solo,
            you: None,
            server_tick: 0,
            key: 0,
            key_issued_tick: 0,
            content_rev: CONTENT_REV,
        }
    }

    pub fn phase(&self) -> MpPhase {
        self.phase
    }

    pub fn begin_connect(&mut self) {
        self.phase = MpPhase::Connecting;
        self.you = None;
    }

    pub fn accept_welcome(
        &mut self,
        you: PlayerId,
        tick: Tick,
        key: SessionKey,
        issued_tick: Tick,
        content_rev: u32,
    ) {
        self.phase = MpPhase::Joined;
        self.you = Some(you);
        self.server_tick = tick;
        self.key = key;
        self.key_issued_tick = issued_tick;
        self.content_rev = content_rev;
    }

    /// Copy key from a snapshot (server recycle).
    pub fn apply_key(&mut self, key: SessionKey, issued_tick: Tick, tick: Tick) {
        self.key = key;
        self.key_issued_tick = issued_tick;
        self.server_tick = tick;
    }

    pub fn leave_to_solo(&mut self) {
        *self = Self::solo();
    }
}
