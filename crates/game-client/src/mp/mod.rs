//! Client multiplayer mode (`mp/`).
//!
//! Solo load does not require this module to talk to a server. Join opens
//! transport; while joined the client sends Input eagerly, releases body
//! channels into a **held** intent at server land delay (032), steps body at
//! `TICK_DURATION_SECS` (same cadence as the authority), hard-snaps to Snapshot
//! `you` when needed, buffers remotes (027–029). Look stays session-immediate.

mod inbound;
mod lag;
mod outbound;
mod pose;
mod remotes;
mod session;
mod transport;

pub use inbound::InboundQueue;
pub use outbound::OutboundQueue;
pub use pose::{apply_spawn, pose_to_state, LandSample};
pub use remotes::{RemoteKitKey, RemoteTable};
pub use session::{MpPhase, MpSession};
pub use transport::{default_ws_url, MpTransport, TransportEvent};

use std::collections::VecDeque;

use game_net::{
    ClientToServer, Hello, Input, NetLocomotion, NetPlayerPose, NetSpawn, Seq, ServerToClient,
    CONTENT_REV, PROTOCOL_VERSION, TICK_DURATION_SECS,
};
use game_sim::SelfState;

use lag::LagEstimator;
use pose::apply_body_from_pose;

/// Cap land-queue history (enough for RTT at high send rate).
const LAND_HISTORY_CAP: usize = 256;

/// Count `corr` only for teleports / serious desync (metres), not normal ack snaps.
const CORR_TELEPORT_M: f32 = 1.0;

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

/// Pending authority sample for hard reconcile.
struct PendingYou {
    pose: NetPlayerPose,
    ack_seq: Seq,
}

/// Body channels after land; stepped once per authority tick (mirrors server hold).
#[derive(Debug, Clone, Copy, Default)]
struct HeldBody {
    wish_forward: f32,
    wish_strafe: f32,
    pending_jump: bool,
    pending_sprint_tap: bool,
    pending_weapon_cycle: i8,
}

impl HeldBody {
    fn absorb(&mut self, input: &Input) {
        self.wish_forward = input.wish_forward;
        self.wish_strafe = input.wish_strafe;
        self.pending_jump |= input.jump;
        self.pending_sprint_tap |= input.sprint_tap;
        if input.weapon_cycle != 0 {
            self.pending_weapon_cycle = input.weapon_cycle;
        }
    }

    /// One server-equivalent tick. Edges fire at most once per tick.
    fn step_tick(state: &mut SelfState, held: &mut Self) {
        if held.pending_jump {
            state.try_jump();
            held.pending_jump = false;
        }
        if held.pending_weapon_cycle != 0 {
            state.cycle_weapon(held.pending_weapon_cycle);
            held.pending_weapon_cycle = 0;
        }
        let sprint_tap = held.pending_sprint_tap;
        held.pending_sprint_tap = false;
        state.apply_move(
            TICK_DURATION_SECS,
            held.wish_forward,
            held.wish_strafe,
            sprint_tap,
        );
    }
}

/// Client multiplayer facade. Default phase is solo (no socket).
pub struct MpClient {
    pub session: MpSession,
    pub transport: MpTransport,
    pub inbound: InboundQueue,
    pub outbound: OutboundQueue,
    pub remotes: RemoteTable,
    lag: LagEstimator,
    pending_spawn: Option<NetSpawn>,
    pending_you: Option<PendingYou>,
    land_history: VecDeque<LandSample>,
    /// Highest seq absorbed into [`HeldBody`].
    absorbed_seq: Seq,
    /// Fixed-step accumulator for joined body (seconds).
    body_tick_acc: f32,
    held: HeldBody,
    input_seq: u32,
    last_reject: Option<String>,
    land_delay_secs: f32,
    uplink_secs: f32,
    l_min_secs: f32,
    stall_secs: f32,
    land_err_ema: f32,
    corr_times: VecDeque<f64>,
}

impl MpClient {
    pub fn new() -> Self {
        Self {
            session: MpSession::solo(),
            transport: MpTransport::new(),
            inbound: InboundQueue::new(),
            outbound: OutboundQueue::new(),
            remotes: RemoteTable::new(),
            lag: LagEstimator::new(),
            pending_spawn: None,
            pending_you: None,
            land_history: VecDeque::new(),
            absorbed_seq: 0,
            body_tick_acc: 0.0,
            held: HeldBody::default(),
            input_seq: 0,
            last_reject: None,
            land_delay_secs: TICK_DURATION_SECS,
            uplink_secs: 0.0,
            l_min_secs: 0.0,
            stall_secs: TICK_DURATION_SECS,
            land_err_ema: 0.0,
            corr_times: VecDeque::new(),
        }
    }

    fn reset_land_metrics(&mut self) {
        self.land_delay_secs = TICK_DURATION_SECS;
        self.uplink_secs = 0.0;
        self.l_min_secs = 0.0;
        self.stall_secs = TICK_DURATION_SECS;
        self.land_err_ema = 0.0;
        self.corr_times.clear();
        self.absorbed_seq = 0;
        self.body_tick_acc = 0.0;
        self.held = HeldBody::default();
    }

    fn reset_lag(&mut self) {
        self.lag.clear();
        self.remotes.set_interp_delay_secs(self.lag.delay_secs());
        self.reset_land_metrics();
    }

    pub fn joined(&self) -> bool {
        self.session.phase() == MpPhase::Joined
    }

    pub fn begin_join(&mut self, url: &str) -> Result<(), wasm_bindgen::JsValue> {
        self.leave_soft();
        self.remotes.clear();
        self.reset_lag();
        self.pending_spawn = None;
        self.pending_you = None;
        self.land_history.clear();
        self.last_reject = None;
        self.input_seq = 0;
        self.session.begin_connect();
        self.transport.connect(url)
    }

    pub fn begin_join_default(&mut self) -> Result<(), wasm_bindgen::JsValue> {
        let url = default_ws_url()?;
        self.begin_join(&url)
    }

    pub fn leave(&mut self) {
        self.leave_soft();
        self.session.leave_to_solo();
        self.remotes.clear();
        self.reset_lag();
        self.pending_spawn = None;
        self.pending_you = None;
        self.land_history.clear();
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
            MpPhase::Joined => {
                let delay_ms = (self.remotes.interp_delay_secs() * 1000.0).round() as i32;
                let rtt = match self.lag.rtt_ema() {
                    Some(s) => format!(" rtt={:.0}ms", s * 1000.0),
                    None => String::new(),
                };
                format!(
                    "mp: joined id={} tick={} key={:#x} remotes={} rdelay={}ms land={:.0}ms{}",
                    self.session.you.unwrap_or(0),
                    self.session.server_tick,
                    self.session.key,
                    self.remotes.count(),
                    delay_ms,
                    self.land_delay_secs * 1000.0,
                    rtt
                )
            }
        }
    }

    pub fn net_hud_fields(&self) -> String {
        match self.session.phase() {
            MpPhase::Solo => "solo".into(),
            MpPhase::Connecting => "connecting".into(),
            MpPhase::Joined => {
                let rdelay_ms = (self.lag.delay_secs() * 1000.0).round() as i32;
                let rtt = match self.lag.rtt_ema() {
                    Some(s) => format!("{:.0}ms", s * 1000.0),
                    None => "—".into(),
                };
                let now = lag::client_now_secs();
                let corr = self.corr_count_per_sec(now);
                format!(
                    "rtt {}  rdelay {}ms  tick {}  Lmin {:.0} Lme {:.0} land {:.0} stall {:.0} err {:.1} corr {:.0}",
                    rtt,
                    rdelay_ms,
                    self.session.server_tick,
                    self.l_min_secs * 1000.0,
                    self.uplink_secs * 1000.0,
                    self.land_delay_secs * 1000.0,
                    self.stall_secs * 1000.0,
                    self.land_err_ema * 1000.0,
                    corr
                )
            }
        }
    }

    fn corr_count_per_sec(&self, now: f64) -> f32 {
        self.corr_times.iter().filter(|t| now - **t <= 1.0).count() as f32
    }

    fn note_correct(&mut self, now: f64) {
        self.corr_times.push_back(now);
        while self.corr_times.front().is_some_and(|t| now - *t > 1.0) {
            self.corr_times.pop_front();
        }
    }

    /// Eager-send Input; land into held body; step at authority tick rate.
    pub fn push_input_land(&mut self, state: &mut SelfState, intent: &InputIntent, frame_dt: f32) {
        if !self.joined() {
            return;
        }
        let now = lag::client_now_secs();
        self.input_seq = self.input_seq.wrapping_add(1);
        let stamp = now;
        let input = Input {
            seq: self.input_seq,
            echo_key: self.session.key,
            echo_issued_tick: self.session.key_issued_tick,
            intent_stamp_secs: stamp,
            wish_forward: intent.wish_forward,
            wish_strafe: intent.wish_strafe,
            look_yaw: intent.look_yaw,
            look_pitch: intent.look_pitch,
            jump: intent.jump,
            sprint_tap: intent.sprint_tap,
            weapon_cycle: intent.weapon_cycle,
        };
        self.stall_secs = self.land_delay_secs;
        let land_at = stamp + f64::from(self.land_delay_secs);
        self.land_history.push_back(LandSample {
            input: input.clone(),
            land_at_secs: land_at,
        });
        while self.land_history.len() > LAND_HISTORY_CAP {
            self.land_history.pop_front();
        }
        self.lag.note_input_sent(input.seq, stamp);
        self.outbound.push(ClientToServer::Input(input));

        self.advance_joined_body(state, frame_dt, now);
    }

    /// Absorb due samples into held, then fixed-step body like the server tick.
    fn advance_joined_body(&mut self, state: &mut SelfState, frame_dt: f32, now: f64) {
        self.absorb_due_samples(now);

        let tick = TICK_DURATION_SECS;
        self.body_tick_acc += frame_dt.max(0.0);
        // Cap catch-up so a tab hitch does not simulate seconds of motion.
        let max_steps = 8u32;
        let mut steps = 0u32;
        while self.body_tick_acc >= tick && steps < max_steps {
            HeldBody::step_tick(state, &mut self.held);
            self.body_tick_acc -= tick;
            steps += 1;
        }
        if steps == max_steps {
            self.body_tick_acc = self.body_tick_acc.min(tick);
        }
    }

    fn absorb_due_samples(&mut self, now: f64) {
        let mut due: Vec<(Seq, Input, f64)> = Vec::new();
        for sample in &self.land_history {
            if sample.input.seq <= self.absorbed_seq {
                continue;
            }
            if sample.land_at_secs > now {
                continue;
            }
            due.push((sample.input.seq, sample.input.clone(), sample.land_at_secs));
        }
        due.sort_by_key(|(seq, _, _)| *seq);
        for (seq, input, land_at) in due {
            let err = (now - land_at).abs() as f32;
            if err.is_finite() {
                // How late we absorbed vs scheduled land (timing quality, not body snap).
                self.land_err_ema += 0.15 * (err - self.land_err_ema);
            }
            self.held.absorb(&input);
            self.absorbed_seq = seq;
        }
    }

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
                        self.reset_lag();
                        self.pending_you = None;
                        self.land_history.clear();
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
                self.land_history.clear();
                self.absorbed_seq = 0;
                self.body_tick_acc = 0.0;
                self.held = HeldBody::default();
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
                self.lag.on_ack(s.ack_seq, lag::client_now_secs());
                self.remotes.set_interp_delay_secs(self.lag.delay_secs());
                self.land_delay_secs = s.land_delay_secs.max(TICK_DURATION_SECS);
                self.uplink_secs = s.uplink_secs.max(0.0);
                self.l_min_secs = s.l_min_secs.max(0.0);
                self.stall_secs = self.land_delay_secs;
                if let Some(you) = s.you {
                    self.pending_you = Some(PendingYou {
                        pose: you,
                        ack_seq: s.ack_seq,
                    });
                } else {
                    self.land_history
                        .retain(|sample| sample.input.seq > s.ack_seq);
                }
                self.remotes.apply_snapshot_others(s.tick, s.others);
            }
            ServerToClient::PlayerLeft(left) => {
                self.remotes.remove(left.id);
            }
        }
    }

    /// Apply spawn; adopt Snapshot `you` as body baseline (032).
    pub fn apply_authority_to_self(&mut self, state: &mut SelfState) {
        if let Some(spawn) = self.take_pending_spawn() {
            apply_spawn(state, &spawn);
            self.land_history.clear();
            self.absorbed_seq = 0;
            self.body_tick_acc = 0.0;
            self.held = HeldBody::default();
        }
        if let Some(PendingYou { pose, ack_seq }) = self.pending_you.take() {
            let now = lag::client_now_secs();
            self.land_history
                .retain(|sample| sample.input.seq > ack_seq);

            // Ack baseline: adopt `you` every Snapshot. Local body only *predicts*
            // between snapshots via tick-step + held (same cadence as server).
            // Do not compare live (ahead by unacked predict) to bare `you` — that
            // is always "wrong" while moving and was the corr spam.
            let dx = state.position.x - pose.position.x;
            let dy = state.position.y - pose.position.y;
            let dz = state.position.z - pose.position.z;
            let err = (dx * dx + dy * dy + dz * dz).sqrt();

            let yaw = state.ocular_yaw;
            let pitch = state.ocular_pitch;
            apply_body_from_pose(state, &pose);
            state.set_look(yaw, pitch);

            if matches!(pose.locomotion, NetLocomotion::Air) {
                self.held.pending_jump = false;
            }
            if matches!(pose.locomotion, NetLocomotion::Sprint) {
                self.held.pending_sprint_tap = false;
            }
            // Re-predict from this baseline until the next Snapshot.
            self.body_tick_acc = 0.0;

            if err > CORR_TELEPORT_M {
                self.note_correct(now);
            }

            self.absorbed_seq = self.absorbed_seq.max(ack_seq);
            if let Some(last) = self
                .land_history
                .iter()
                .rev()
                .find(|s| s.land_at_secs <= now)
            {
                self.held.wish_forward = last.input.wish_forward;
                self.held.wish_strafe = last.input.wish_strafe;
            }
        }
        if self.joined() {
            // No extra frame_dt here — body steps in push_input_land.
            // Still absorb any due samples if input path did not run this frame.
            let now = lag::client_now_secs();
            self.absorb_due_samples(now);
        }
    }
}

impl Default for MpClient {
    fn default() -> Self {
        Self::new()
    }
}
