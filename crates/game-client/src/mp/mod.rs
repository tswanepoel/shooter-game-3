//! Client multiplayer mode (`mp/`).
//!
//! Solo load does not require this module to talk to a server. Join (023) opens
//! transport, yields authority, and fills remotes from snapshots.

mod inbound;
mod outbound;
mod remotes;
mod session;
mod transport;

pub use inbound::InboundQueue;
pub use outbound::OutboundQueue;
pub use remotes::RemoteTable;
pub use session::{MpPhase, MpSession};
pub use transport::{default_ws_url, MpTransport, TransportEvent};

/// Client multiplayer facade. Default phase is solo (no socket).
pub struct MpClient {
    pub session: MpSession,
    pub transport: MpTransport,
    pub inbound: InboundQueue,
    pub outbound: OutboundQueue,
    pub remotes: RemoteTable,
}

impl MpClient {
    pub fn new() -> Self {
        Self {
            session: MpSession::solo(),
            transport: MpTransport::new(),
            inbound: InboundQueue::new(),
            outbound: OutboundQueue::new(),
            remotes: RemoteTable::new(),
        }
    }

    /// True when local self should advance from server snapshots.
    pub fn joined(&self) -> bool {
        self.session.phase() == MpPhase::Joined
    }

    /// Open WebSocket and move to Connecting (023 wires devtools here).
    pub fn begin_join(&mut self, url: &str) -> Result<(), wasm_bindgen::JsValue> {
        self.remotes.clear();
        self.session.begin_connect();
        self.transport.connect(url)
    }

    /// Join using `ws://{page-host}:9090/`.
    pub fn begin_join_default(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        let url = default_ws_url()?;
        self.begin_join(&url)
    }

    /// Close socket and return to solo.
    pub fn leave(&mut self) {
        self.transport.close();
        self.session.leave_to_solo();
        self.remotes.clear();
        while self.inbound.pop().is_some() {}
        while self.outbound.pop_discard() {}
    }

    /// Drain transport → inbound → session/remotes; flush outbound → socket.
    pub fn poll_transport(&mut self) {
        for ev in self.transport.poll_events() {
            match ev {
                TransportEvent::Binary(bytes) => self.inbound.push_bytes(&bytes),
                TransportEvent::Open => {
                    if self.session.phase() == MpPhase::Connecting {
                        self.outbound
                            .push(game_net::ClientToServer::Hello(game_net::Hello {
                                protocol: game_net::PROTOCOL_VERSION,
                                content_rev: game_net::CONTENT_REV,
                            }));
                    }
                }
                TransportEvent::Close | TransportEvent::Error => {
                    if self.session.phase() != MpPhase::Solo {
                        self.session.leave_to_solo();
                        self.remotes.clear();
                    }
                }
            }
        }

        while let Some(msg) = self.inbound.pop() {
            self.apply_s2c(msg);
        }

        if !self.transport.connected() {
            return;
        }
        if let Ok(frames) = self.outbound.drain_encoded() {
            for bytes in frames {
                let _ = self.transport.send_binary(&bytes);
            }
        }
    }

    fn apply_s2c(&mut self, msg: game_net::ServerToClient) {
        use game_net::ServerToClient;
        match msg {
            ServerToClient::Welcome(w) => {
                self.session
                    .accept_welcome(w.you, w.tick, w.key, w.issued_tick, w.content_rev);
            }
            ServerToClient::Reject(_) => {
                self.leave();
            }
            ServerToClient::Snapshot(s) => {
                if !self.joined() {
                    return;
                }
                self.session.apply_key(s.key, s.issued_tick, s.tick);
                // Local `you` pose apply is 023; remotes table fill is 024-ready.
                self.remotes.clear();
                for pose in s.others {
                    self.remotes.insert(pose);
                }
            }
            ServerToClient::PlayerLeft(left) => {
                self.remotes.remove(left.id);
            }
        }
    }
}

impl Default for MpClient {
    fn default() -> Self {
        Self::new()
    }
}
