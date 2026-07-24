//! WebTransport multiplayer session.
//!
//! Join/leave is driven by the debug shell today. Release builds (`--no-default-features`)
//! still compile session frame hooks (drive, claims, remotes) but have no join UI, so the
//! join path is unused without `debug-tools`.
#![cfg_attr(not(feature = "debug-tools"), allow(dead_code))]

mod clock;
mod drive;
mod remotes;

pub use drive::{drive_to_state, state_to_drive};
pub use remotes::{RemoteKitKey, RemoteTable};

use std::cell::RefCell;
use std::rc::Rc;

use game_net::{
    decode_s2c, drain_s2c_frames, encode_c2s, ClientToServer, NetProjectileSpawn, PlayerId,
    ServerToClient, PROTOCOL_VERSION, TICK_HZ,
};
use game_sim::{weapon_def, Projectile, SelfState};
use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ReadableStreamDefaultReader, WritableStreamDefaultWriter};

use clock::ClockSync;

const IDENTITY_PATH: &str = "/__debug/wt-identity";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpPhase {
    Solo,
    Connecting,
    Joined,
}

/// Peer projectile batch accepted from the server (038).
#[derive(Debug, Clone)]
pub struct PeerProjectileBatch {
    pub id: PlayerId,
    pub projectiles: Vec<NetProjectileSpawn>,
}

struct Shared {
    phase: MpPhase,
    clock: ClockSync,
    player_id: Option<PlayerId>,
    dgram_writer: Option<WritableStreamDefaultWriter>,
    transport: Option<JsValue>,
    last_error: Option<String>,
    probe_accum: f32,
    drive_accum: f32,
    join_secs: f32,
    remotes: RemoteTable,
    /// Inbound peer projectile claims to apply on the main frame.
    pending_projectiles: Vec<PeerProjectileBatch>,
}

impl Shared {
    fn new() -> Self {
        Self {
            phase: MpPhase::Solo,
            clock: ClockSync::new(),
            player_id: None,
            dgram_writer: None,
            transport: None,
            last_error: None,
            probe_accum: 0.0,
            drive_accum: 0.0,
            join_secs: 0.0,
            remotes: RemoteTable::new(),
            pending_projectiles: Vec::new(),
        }
    }

    fn reset_session(&mut self) {
        self.phase = MpPhase::Solo;
        self.clock.clear();
        self.player_id = None;
        self.dgram_writer = None;
        self.transport = None;
        self.probe_accum = 0.0;
        self.drive_accum = 0.0;
        self.join_secs = 0.0;
        self.remotes.clear();
        self.pending_projectiles.clear();
    }
}

pub struct MpClient {
    shared: Rc<RefCell<Shared>>,
}

impl MpClient {
    pub fn new() -> Self {
        Self {
            shared: Rc::new(RefCell::new(Shared::new())),
        }
    }

    pub fn joined(&self) -> bool {
        self.shared.borrow().phase == MpPhase::Joined
    }

    pub fn remotes(&self) -> std::cell::Ref<'_, RemoteTable> {
        std::cell::Ref::map(self.shared.borrow(), |s| &s.remotes)
    }

    pub fn status_line(&self) -> String {
        let s = self.shared.borrow();
        match s.phase {
            MpPhase::Solo => "mp: solo".into(),
            MpPhase::Connecting => "mp: connecting…".into(),
            MpPhase::Joined => {
                let id = s
                    .player_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "—".into());
                let tick = s
                    .clock
                    .estimated_tick(client_now_secs())
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "—".into());
                let off = s
                    .clock
                    .offset_secs()
                    .map(|o| format!("{:.1}ms", o * 1000.0))
                    .unwrap_or_else(|| "—".into());
                let delay = s
                    .clock
                    .last_delay_secs()
                    .map(|d| format!("{:.1}ms", d * 1000.0))
                    .unwrap_or_else(|| "—".into());
                format!(
                    "mp: joined id={id} tick={tick} remotes={} offset={off} delay={delay} samples={}",
                    s.remotes.count(),
                    s.clock.sample_count()
                )
            }
        }
    }

    pub fn hud_tick_field(&self) -> Option<String> {
        let s = self.shared.borrow();
        if s.phase != MpPhase::Joined {
            return None;
        }
        let tick = s
            .clock
            .estimated_tick(client_now_secs())
            .map(|t| t.to_string())
            .unwrap_or_else(|| "—".into());
        Some(format!("tick {tick}"))
    }

    pub fn take_error(&mut self) -> Option<String> {
        self.shared.borrow_mut().last_error.take()
    }

    pub fn player_id(&self) -> Option<PlayerId> {
        self.shared.borrow().player_id
    }

    /// Drain accepted peer projectile claims (038).
    pub fn take_peer_projectiles(&mut self) -> Vec<PeerProjectileBatch> {
        std::mem::take(&mut self.shared.borrow_mut().pending_projectiles)
    }

    /// Claim local projectile spawns to the server (joined only).
    pub fn claim_projectiles(&self, projectiles: &[Projectile]) {
        if projectiles.is_empty() {
            return;
        }
        let s = self.shared.borrow();
        if s.phase != MpPhase::Joined {
            return;
        }
        let Some(writer) = s.dgram_writer.as_ref() else {
            return;
        };
        let tick = s.clock.estimated_tick(client_now_secs()).unwrap_or(0);
        let spawns: Vec<NetProjectileSpawn> = projectiles
            .iter()
            .map(|p| NetProjectileSpawn {
                id: p.id,
                weapon: p.weapon,
                origin: game_net::NetVec3::new(p.origin.x, p.origin.y, p.origin.z),
                velocity: game_net::NetVec3::new(p.velocity.x, p.velocity.y, p.velocity.z),
                muzzle_index: p.muzzle_index,
            })
            .collect();
        let Ok(payload) = encode_c2s(&ClientToServer::ProjectileSpawn {
            tick,
            projectiles: spawns,
        }) else {
            return;
        };
        let arr = Uint8Array::from(payload.as_slice());
        let _ = writer.write_with_chunk(&arr);
    }

    pub fn begin_join(&self) {
        let mut s = self.shared.borrow_mut();
        if s.phase != MpPhase::Solo {
            return;
        }
        s.phase = MpPhase::Connecting;
        s.clock.clear();
        s.player_id = None;
        s.remotes.clear();
        s.last_error = None;
        drop(s);

        let shared = Rc::clone(&self.shared);
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = join_session(Rc::clone(&shared)).await {
                let msg = js_error_string(&e);
                let mut s = shared.borrow_mut();
                s.reset_session();
                s.last_error = Some(format!("mp: join failed: {msg}"));
            }
        });
    }

    pub fn leave(&self) {
        let mut s = self.shared.borrow_mut();
        if let Some(t) = s.transport.take() {
            if let Ok(close) = Reflect::get(&t, &"close".into()) {
                if let Ok(f) = close.dyn_into::<js_sys::Function>() {
                    let _ = f.call0(&t);
                }
            }
        }
        s.reset_session();
    }

    pub fn on_frame(&mut self, dt: f32, self_state: &SelfState) {
        let mut s = self.shared.borrow_mut();
        if s.phase != MpPhase::Joined {
            return;
        }
        s.join_secs += dt;
        s.probe_accum += dt;
        s.drive_accum += dt;

        let writer = match s.dgram_writer.as_ref() {
            Some(w) => w.clone(),
            None => return,
        };

        let probe_interval = if s.join_secs < 1.0 { 0.05 } else { 0.2 };
        let send_probe = s.probe_accum >= probe_interval;
        if send_probe {
            s.probe_accum = 0.0;
        }

        let drive_interval = 1.0 / TICK_HZ as f32;
        let send_drive = s.drive_accum >= drive_interval;
        let drive_payload = if send_drive {
            s.drive_accum = 0.0;
            let tick = s.clock.estimated_tick(client_now_secs()).unwrap_or(0);
            let drive = state_to_drive(self_state);
            encode_c2s(&ClientToServer::DriveSample { tick, drive }).ok()
        } else {
            None
        };
        drop(s);

        if send_probe {
            let t1 = client_now_secs();
            if let Ok(payload) = encode_c2s(&ClientToServer::ClockProbe { t1 }) {
                let arr = Uint8Array::from(payload.as_slice());
                let _ = writer.write_with_chunk(&arr);
            }
        }

        if let Some(payload) = drive_payload {
            let arr = Uint8Array::from(payload.as_slice());
            let _ = writer.write_with_chunk(&arr);
        }
    }
}

impl Default for MpClient {
    fn default() -> Self {
        Self::new()
    }
}

fn handle_s2c(shared: &Rc<RefCell<Shared>>, msg: ServerToClient, t4: f64) {
    match msg {
        ServerToClient::ClockReply { t1, t2, t3, tick } => {
            shared.borrow_mut().clock.on_sample(t1, t2, t3, t4, tick);
        }
        ServerToClient::PeerJoined { id, .. } => {
            let mut s = shared.borrow_mut();
            if s.player_id == Some(id) {
                return;
            }
            s.remotes.note_joined(id);
        }
        ServerToClient::PeerLeft { id, .. } => {
            shared.borrow_mut().remotes.remove(id);
        }
        ServerToClient::PeerDrive { tick, id, drive } => {
            let mut s = shared.borrow_mut();
            if s.player_id == Some(id) {
                return;
            }
            s.remotes.upsert_drive(id, tick, drive);
        }
        ServerToClient::PeerProjectileSpawn {
            id, projectiles, ..
        } => {
            let mut s = shared.borrow_mut();
            if s.player_id == Some(id) || projectiles.is_empty() {
                return;
            }
            s.pending_projectiles
                .push(PeerProjectileBatch { id, projectiles });
        }
        ServerToClient::Welcome { .. } | ServerToClient::Reject { .. } => {}
    }
}

/// Convert a net spawn into a sim projectile (ammo + max range from weapon table).
pub fn net_spawn_to_projectile(owner: PlayerId, n: &NetProjectileSpawn) -> Option<Projectile> {
    let def = weapon_def(n.weapon)?;
    let origin = glam::Vec3::new(n.origin.x, n.origin.y, n.origin.z);
    let velocity = glam::Vec3::new(n.velocity.x, n.velocity.y, n.velocity.z);
    Some(Projectile {
        id: n.id,
        owner,
        weapon: n.weapon,
        ammo: def.ammo(),
        origin,
        position: origin,
        velocity,
        traveled: 0.0,
        max_range: def.max_range,
        muzzle_index: n.muzzle_index,
    })
}

async fn join_session(shared: Rc<RefCell<Shared>>) -> Result<(), JsValue> {
    let identity = fetch_identity().await?;
    let transport = open_webtransport(&identity.url, &identity.hash_sha256)?;
    let ready = Reflect::get(&transport, &"ready".into())?;
    JsFuture::from(js_sys::Promise::from(ready)).await?;

    let create_bi = Reflect::get(&transport, &"createBidirectionalStream".into())?
        .dyn_into::<js_sys::Function>()?;
    let bi = JsFuture::from(js_sys::Promise::from(create_bi.call0(&transport)?)).await?;
    let writable: web_sys::WritableStream = Reflect::get(&bi, &"writable".into())?.dyn_into()?;
    let writer = writable.get_writer()?;
    let hello = encode_c2s(&ClientToServer::Hello {
        protocol: PROTOCOL_VERSION,
    })
    .map_err(|e| JsValue::from_str(&format!("encode Hello: {e}")))?;
    JsFuture::from(writer.write_with_chunk(&Uint8Array::from(hello.as_slice()))).await?;

    let readable: web_sys::ReadableStream = Reflect::get(&bi, &"readable".into())?.dyn_into()?;
    let reader: ReadableStreamDefaultReader = readable
        .get_reader()
        .dyn_into()
        .map_err(|_| JsValue::from_str("bi.readable reader"))?;
    let read = JsFuture::from(reader.read()).await?;
    if Reflect::get(&read, &"done".into())?
        .as_bool()
        .unwrap_or(true)
    {
        return Err(JsValue::from_str("stream closed before Welcome"));
    }
    let bytes = Uint8Array::new(&Reflect::get(&read, &"value".into())?).to_vec();
    let s2c = decode_s2c(&bytes).map_err(|_| JsValue::from_str("decode S2C failed"))?;
    let (player_id, tick, server_time) = match s2c {
        ServerToClient::Reject { reason } => {
            return Err(JsValue::from_str(&format!("rejected: {reason}")));
        }
        ServerToClient::Welcome {
            protocol,
            player_id,
            tick,
            server_time_secs,
        } => {
            if protocol != PROTOCOL_VERSION {
                return Err(JsValue::from_str(&format!("protocol mismatch: {protocol}")));
            }
            (player_id, tick, server_time_secs)
        }
        _ => {
            return Err(JsValue::from_str("expected Welcome"));
        }
    };

    let datagrams = Reflect::get(&transport, &"datagrams".into())?;
    let dgram_writable: web_sys::WritableStream =
        Reflect::get(&datagrams, &"writable".into())?.dyn_into()?;
    let dgram_writer = dgram_writable.get_writer()?;
    let dgram_readable: web_sys::ReadableStream =
        Reflect::get(&datagrams, &"readable".into())?.dyn_into()?;
    let dgram_reader: ReadableStreamDefaultReader = dgram_readable
        .get_reader()
        .dyn_into()
        .map_err(|_| JsValue::from_str("datagram reader"))?;

    {
        let mut s = shared.borrow_mut();
        s.transport = Some(transport.clone());
        s.dgram_writer = Some(dgram_writer);
        s.player_id = Some(player_id);
        s.clock
            .seed_from_welcome(client_now_secs(), server_time, tick);
        s.phase = MpPhase::Joined;
        s.join_secs = 0.0;
        s.probe_accum = 0.0;
        s.drive_accum = 0.0;
        s.remotes.clear();
    }

    let shared_bi = Rc::clone(&shared);
    wasm_bindgen_futures::spawn_local(async move {
        let mut frame_buf: Vec<u8> = Vec::new();
        loop {
            let read = match JsFuture::from(reader.read()).await {
                Ok(v) => v,
                Err(_) => break,
            };
            let done = Reflect::get(&read, &"done".into())
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if done {
                break;
            }
            let value = match Reflect::get(&read, &"value".into()) {
                Ok(v) => v,
                Err(_) => break,
            };
            frame_buf.extend_from_slice(&Uint8Array::new(&value).to_vec());
            let t = client_now_secs();
            for msg in drain_s2c_frames(&mut frame_buf) {
                handle_s2c(&shared_bi, msg, t);
            }
        }
    });

    let shared_dgram = Rc::clone(&shared);
    wasm_bindgen_futures::spawn_local(async move {
        loop {
            let read = match JsFuture::from(dgram_reader.read()).await {
                Ok(v) => v,
                Err(_) => break,
            };
            let done = Reflect::get(&read, &"done".into())
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if done {
                break;
            }
            let value = match Reflect::get(&read, &"value".into()) {
                Ok(v) => v,
                Err(_) => break,
            };
            let bytes = Uint8Array::new(&value).to_vec();
            let t4 = client_now_secs();
            if let Ok(msg) = decode_s2c(&bytes) {
                handle_s2c(&shared_dgram, msg, t4);
            }
        }
        let mut s = shared_dgram.borrow_mut();
        if s.phase == MpPhase::Joined {
            s.reset_session();
        }
    });

    Ok(())
}

struct IdentityDoc {
    url: String,
    hash_sha256: Vec<u8>,
}

async fn fetch_identity() -> Result<IdentityDoc, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let resp_val = JsFuture::from(window.fetch_with_str(IDENTITY_PATH)).await?;
    let resp: web_sys::Response = resp_val.dyn_into()?;
    if !resp.ok() {
        return Err(JsValue::from_str(
            "wt-identity missing (is game-server running?)",
        ));
    }
    let json = JsFuture::from(resp.json()?).await?;
    let url = Reflect::get(&json, &"url".into())?
        .as_string()
        .ok_or_else(|| JsValue::from_str("identity.url"))?;
    let hash_val = Reflect::get(&json, &"hash_sha256".into())?;
    let hash_arr = Array::from(&hash_val);
    let mut hash_sha256 = Vec::with_capacity(32);
    for i in 0..hash_arr.length() {
        let n = hash_arr.get(i).as_f64().unwrap_or(0.0) as u8;
        hash_sha256.push(n);
    }
    if hash_sha256.len() != 32 {
        return Err(JsValue::from_str("identity.hash_sha256 must be 32 bytes"));
    }
    Ok(IdentityDoc { url, hash_sha256 })
}

fn open_webtransport(url: &str, hash: &[u8]) -> Result<JsValue, JsValue> {
    let tight = Uint8Array::new_with_length(hash.len() as u32);
    tight.copy_from(hash);
    let buffer = tight.buffer();

    let hash_entry = Object::new();
    Reflect::set(&hash_entry, &"algorithm".into(), &"sha-256".into())?;
    Reflect::set(&hash_entry, &"value".into(), &buffer)?;

    let hashes = Array::new();
    hashes.push(&hash_entry);

    let options = Object::new();
    Reflect::set(&options, &"serverCertificateHashes".into(), &hashes)?;

    let global = js_sys::global();
    let ctor = Reflect::get(&global, &"WebTransport".into())?;
    let ctor = ctor
        .dyn_into::<js_sys::Function>()
        .map_err(|_| JsValue::from_str("WebTransport not available in this browser"))?;
    let args = Array::new();
    args.push(&JsValue::from_str(url));
    args.push(&options);
    Reflect::construct(&ctor, &args)
}

fn client_now_secs() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now() / 1000.0)
        .unwrap_or(0.0)
}

fn js_error_string(v: &JsValue) -> String {
    if let Some(s) = v.as_string() {
        return s;
    }
    js_sys::JSON::stringify(v)
        .ok()
        .and_then(|s| s.as_string())
        .unwrap_or_else(|| format!("{v:?}"))
}
