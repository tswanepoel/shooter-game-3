//! Client multiplayer mode (`mp/`).
//!
//! Solo load does not require this module to talk to a server. Join opens
//! transport; while joined the client predicts self (026), hard-corrects from
//! Snapshot + `ack_seq`, and buffers `others` with a frame present clock (027 / 028).

mod inbound;
mod outbound;
mod pose;
mod remotes;
mod session;
mod transport;

pub use inbound::InboundQueue;
pub use outbound::OutboundQueue;
pub use pose::{apply_spawn, pose_to_state, predict_intent, reconcile_predicted, PredictedSample};
pub use remotes::{RemoteKitKey, RemoteTable};
pub use session::{MpPhase, MpSession};
pub use transport::{default_ws_url, MpTransport, TransportEvent};

use std::collections::VecDeque;

use game_net::{
    ClientToServer, Hello, Input, NetPlayerPose, NetSpawn, Seq, ServerToClient, CONTENT_REV,
    PROTOCOL_VERSION,
};
use game_sim::SelfState;

/// Cap predicted Input history (enough for RTT at high send rate).
const PREDICT_HISTORY_CAP: usize = 256;

/// One frame of movement intent for C→S Input.
#[derive(Debug, Clone, Copy)]
pub struct InputIntent {
    pub wish_forward: f32,
    pub wish_strafe: f32,
    pub look_yaw: f32,
    pub look_pitch: f32,
    pub jump: bool,
    pub sprint_tap: bool,
    pub weapon_cycle: i8,
}

impl InputIntent {
    pub fn idle_look(yaw: f32, pitch: f32) -> Self {
        Self {
            wish_forward: 0.0,
            wish_strafe: 0.0,
            look_yaw: yaw,
            look_pitch: pitch,
            jump: false,
            sprint_tap: false,
            weapon_cycle: 0,
        }
    }
}

/// Pending authority sample for hard reconcile (026).
struct PendingYou {
    pose: NetPlayerPose,
    ack_seq: Seq,
}

/// Client multiplayer facade. Default phase is solo (no socket).
pub struct MpClient {
    pub session: MpSession,
    pub transport: MpTransport,
    pub inbound: InboundQueue,
    pub outbound: OutboundQueue,
    pub remotes: RemoteTable,
    /// Spawn from Welcome; applied once to local self.
    pending_spawn: Option<NetSpawn>,
    /// Latest authoritative `you` + ack for hard reconcile.
    pending_you: Option<PendingYou>,
    /// Sent Inputs not yet covered by `ack_seq`.
    predict_history: VecDeque<PredictedSample>,
    input_seq: u32,
    last_reject: Option<String>,
}

impl MpClient {
    pub fn new() -> Self {
        Self {
            session: MpSession::solo(),
            transport: MpTransport::new(),
            inbound: InboundQueue::new(),
            outbound: OutboundQueue::new(),
            remotes: RemoteTable::new(),
            pending_spawn: None,
            pending_you: None,
            predict_history: VecDeque::new(),
            input_seq: 0,
            last_reject: None,
        }
    }

    /// True when multiplayer session is joined.
    pub fn joined(&self) -> bool {
        self.session.phase() == MpPhase::Joined
    }

    /// Open WebSocket and move to Connecting.
    pub fn begin_join(&mut self, url: &str) -> Result<(), wasm_bindgen::JsValue> {
        self.leave_soft();
        self.remotes.clear();
        self.pending_spawn = None;
        self.pending_you = None;
        self.predict_history.clear();
        self.last_reject = None;
        self.input_seq = 0;
        self.session.begin_connect();
        self.transport.connect(url)
    }

    /// Join using `ws://{page-host}:9090/`.
    pub fn begin_join_default(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        let url = default_ws_url()?;
        self.begin_join(&url)
    }

    /// Close socket and return to solo (keeps current self pose).
    pub fn leave(&mut self) {
        self.leave_soft();
        self.session.leave_to_solo();
        self.remotes.clear();
        self.pending_spawn = None;
        self.pending_you = None;
        self.predict_history.clear();
        while self.inbound.pop().is_some() {}
        while self.outbound.pop_discard() {}
    }

    fn leave_soft(&mut self) {
        self.transport.close();
    }

    pub fn take_pending_spawn(&mut self) -> Option<NetSpawn> {
        self.pending_spawn.take()
    }

    pub fn take_reject_message(&mut self) -> Option<String> {
        self.last_reject.take()
    }

    pub fn status_line(&self) -> String {
        match self.session.phase() {
            MpPhase::Solo => "mp: solo".into(),
            MpPhase::Connecting => "mp: connecting…".into(),
            MpPhase::Joined => format!(
                "mp: joined id={} tick={} key={:#x} remotes={}",
                self.session.you.unwrap_or(0),
                self.session.server_tick,
                self.session.key,
                self.remotes.count()
            ),
        }
    }

    /// Queue one Input, record predict history, and advance local body (joined only).
    pub fn push_input_predict(&mut self, state: &mut SelfState, intent: &InputIntent, dt: f32) {
        if !self.joined() {
            return;
        }
        self.input_seq = self.input_seq.wrapping_add(1);
        let input = Input {
            seq: self.input_seq,
            echo_key: self.session.key,
            echo_issued_tick: self.session.key_issued_tick,
            wish_forward: intent.wish_forward,
            wish_strafe: intent.wish_strafe,
            look_yaw: intent.look_yaw,
            look_pitch: intent.look_pitch,
            jump: intent.jump,
            sprint_tap: intent.sprint_tap,
            weapon_cycle: intent.weapon_cycle,
        };
        self.predict_history.push_back(PredictedSample {
            input: input.clone(),
            dt,
        });
        while self.predict_history.len() > PREDICT_HISTORY_CAP {
            self.predict_history.pop_front();
        }
        self.outbound.push(ClientToServer::Input(input));
        predict_intent(state, intent, dt);
    }

    /// Drain transport → inbound → session; flush outbound → socket.
    pub fn poll_transport(&mut self) {
        for ev in self.transport.poll_events() {
            match ev {
                TransportEvent::Binary(bytes) => self.inbound.push_bytes(&bytes),
                TransportEvent::Open => {
                    if self.session.phase() == MpPhase::Connecting {
                        self.outbound.push(ClientToServer::Hello(Hello {
                            protocol: PROTOCOL_VERSION,
                            content_rev: CONTENT_REV,
                        }));
                    }
                }
                TransportEvent::Close | TransportEvent::Error => {
                    if self.session.phase() != MpPhase::Solo {
                        self.session.leave_to_solo();
                        self.remotes.clear();
                        self.pending_you = None;
                        self.predict_history.clear();
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

    fn apply_s2c(&mut self, msg: ServerToClient) {
        match msg {
            ServerToClient::Welcome(w) => {
                self.session
                    .accept_welcome(w.you, w.tick, w.key, w.issued_tick, w.content_rev);
                self.pending_spawn = Some(w.spawn);
                self.predict_history.clear();
            }
            ServerToClient::Reject(r) => {
                self.last_reject = Some(format!("mp: rejected ({:?})", r.reason));
                self.leave();
            }
            ServerToClient::Snapshot(s) => {
                if !self.joined() {
                    return;
                }
                self.session.apply_key(s.key, s.issued_tick, s.tick);
                if let Some(you) = s.you {
                    self.pending_you = Some(PendingYou {
                        pose: you,
                        ack_seq: s.ack_seq,
                    });
                } else {
                    // Still drop acked samples if pose omitted.
                    self.predict_history
                        .retain(|sample| sample.input.seq > s.ack_seq);
                }
                self.remotes.apply_snapshot_others(s.tick, s.others);
            }
            ServerToClient::PlayerLeft(left) => {
                self.remotes.remove(left.id);
            }
        }
    }

    /// Apply spawn and hard-reconcile any pending Snapshot `you`.
    pub fn apply_authority_to_self(&mut self, state: &mut SelfState) {
        if let Some(spawn) = self.take_pending_spawn() {
            apply_spawn(state, &spawn);
            self.predict_history.clear();
        }
        if let Some(PendingYou { pose, ack_seq }) = self.pending_you.take() {
            reconcile_predicted(state, &pose, ack_seq, &mut self.predict_history);
        }
    }
}

impl Default for MpClient {
    fn default() -> Self {
        Self::new()
    }
}
