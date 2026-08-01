//! WebTransport join and inbound S2C handling.

use std::cell::RefCell;
use std::rc::Rc;

use game_net::{
    decode_s2c, drain_s2c_frames, encode_c2s_frame, take_s2c_frame, ClientToServer, ServerToClient,
    PROTOCOL_VERSION,
};
use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::ReadableStreamDefaultReader;

use super::apply::{apply_roster, apply_you_spawned};
use super::cookie::save_display_name_cookie;
use super::shared::{BlasterGrantBatch, DeathAnnounceBatch, LootGrantBatch};
use super::{client_now_secs, MpPhase, PeerImpactHitBatch, PeerProjectileBatch, Shared};
use game_sim::prefer_armed_slot;

const IDENTITY_PATH: &str = "/__debug/wt-identity";

pub async fn join_session(shared: Rc<RefCell<Shared>>) -> Result<(), JsValue> {
    let (room_code, display_name) = {
        let s = shared.borrow();
        (s.join_room.clone(), s.join_name.clone())
    };

    let identity = fetch_identity().await?;
    let url = wt_url_from_page(&identity)?;
    let transport = open_webtransport(&url, &identity.hash_sha256)?;
    let ready = Reflect::get(&transport, &"ready".into())?;
    JsFuture::from(js_sys::Promise::from(ready)).await?;

    let create_bi = Reflect::get(&transport, &"createBidirectionalStream".into())?
        .dyn_into::<js_sys::Function>()?;
    let bi = JsFuture::from(js_sys::Promise::from(create_bi.call0(&transport)?)).await?;
    let writable: web_sys::WritableStream = Reflect::get(&bi, &"writable".into())?.dyn_into()?;
    let writer = writable.get_writer()?;
    let hello = encode_c2s_frame(&ClientToServer::Hello {
        protocol: PROTOCOL_VERSION,
        room_code,
        display_name: display_name.clone(),
    })
    .map_err(|e| JsValue::from_str(&format!("encode Hello: {e}")))?;
    JsFuture::from(writer.write_with_chunk(&Uint8Array::from(hello.as_slice()))).await?;

    let readable: web_sys::ReadableStream = Reflect::get(&bi, &"readable".into())?.dyn_into()?;
    let reader: ReadableStreamDefaultReader = readable
        .get_reader()
        .dyn_into()
        .map_err(|_| JsValue::from_str("bi.readable reader"))?;

    // The server may coalesce the roster behind Welcome; keep the tail for the
    // reader task below.
    let mut frame_buf: Vec<u8> = Vec::new();
    let s2c = loop {
        if let Some(msg) =
            take_s2c_frame(&mut frame_buf).map_err(|e| JsValue::from_str(&e.to_string()))?
        {
            break msg;
        }
        let read = JsFuture::from(reader.read()).await?;
        if Reflect::get(&read, &"done".into())?
            .as_bool()
            .unwrap_or(true)
        {
            return Err(JsValue::from_str("stream closed before Welcome"));
        }
        let bytes = Uint8Array::new(&Reflect::get(&read, &"value".into())?).to_vec();
        frame_buf.extend_from_slice(&bytes);
    };
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

    save_display_name_cookie(&display_name);

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
        s.reliable_writer = Some(writer);
        s.player_id = Some(player_id);
        s.display_name = Some(display_name);
        s.clock
            .seed_from_welcome(client_now_secs(), server_time, tick);
        s.phase = MpPhase::Role;
        s.join_secs = 0.0;
        s.probe_accum = 0.0;
        s.drive_accum = 0.0;
        s.spawn_retry_accum = 0.0;
        s.spawn_requested = false;
        s.character = game_net::DEFAULT_CHARACTER;
        s.role = game_net::NetRole::Player;
        s.remotes.clear();
        s.roster.clear();
    }

    let shared_bi = Rc::clone(&shared);
    wasm_bindgen_futures::spawn_local(async move {
        while let Ok(msgs) = drain_s2c_frames(&mut frame_buf) {
            let t = client_now_secs();
            for msg in msgs {
                handle_s2c(&shared_bi, msg, t);
            }
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
        if s.phase.in_room() {
            s.reset_session();
        }
    });

    Ok(())
}

fn handle_s2c(shared: &Rc<RefCell<Shared>>, msg: ServerToClient, t4: f64) {
    match msg {
        ServerToClient::ClockReply { t1, t2, t3, tick } => {
            shared.borrow_mut().clock.on_sample(t1, t2, t3, t4, tick);
        }
        ServerToClient::YouSpawned {
            position, facing, ..
        } => {
            let mut s = shared.borrow_mut();
            let loadout = super::apply::PendingSpawnLoadout {
                primary: s.staged_primary,
                secondary: s.staged_secondary,
                active: prefer_armed_slot(s.staged_primary, s.staged_secondary, s.staged_active),
            };
            let Shared {
                phase,
                spawn_requested,
                pending_spawn,
                ..
            } = &mut *s;
            apply_you_spawned(
                phase,
                spawn_requested,
                pending_spawn,
                position,
                facing,
                loadout,
            );
        }
        ServerToClient::Roster {
            entries,
            match_view,
            ..
        } => {
            let mut s = shared.borrow_mut();
            let player_id = s.player_id;
            {
                let Shared {
                    roster,
                    remotes,
                    match_view: mv,
                    ..
                } = &mut *s;
                *mv = match_view;
                apply_roster(roster, remotes, player_id, entries);
            }
            s.reconcile_self_from_roster();
            s.reconcile_match_setup();
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
        ServerToClient::PeerImpactHit { id, hit, .. } => {
            let mut s = shared.borrow_mut();
            if s.player_id == Some(id) {
                return;
            }
            s.pending_hits.push(PeerImpactHitBatch { hit });
        }
        ServerToClient::DeathAnnounce { victim, killer, .. } => {
            shared
                .borrow_mut()
                .pending_deaths
                .push(DeathAnnounceBatch { victim, killer });
        }
        ServerToClient::CorpseSpawn { corpse, .. } => {
            shared.borrow_mut().pending_corpse_spawns.push(corpse);
        }
        ServerToClient::CorpseEnd { corpse_id, .. } => {
            shared.borrow_mut().pending_corpse_ends.push(corpse_id);
        }
        ServerToClient::AmmoDropSpawn { drop, .. } => {
            shared.borrow_mut().pending_drop_spawns.push(drop);
        }
        ServerToClient::AmmoDropEnd { drop_id, .. } => {
            shared.borrow_mut().pending_drop_ends.push(drop_id);
        }
        ServerToClient::LootGrant {
            drop_id,
            player_id,
            ammo,
            rounds,
            ..
        } => {
            shared
                .borrow_mut()
                .pending_loot_grants
                .push(LootGrantBatch {
                    drop_id,
                    player_id,
                    ammo,
                    rounds,
                });
        }
        ServerToClient::BlasterDropSpawn { drop, .. } => {
            shared.borrow_mut().pending_blaster_drop_spawns.push(drop);
        }
        ServerToClient::BlasterDropEnd { drop_id, .. } => {
            shared.borrow_mut().pending_blaster_drop_ends.push(drop_id);
        }
        ServerToClient::BlasterGrant {
            drop_id,
            player_id,
            letter,
            mag,
            chamber,
            ..
        } => {
            shared
                .borrow_mut()
                .pending_blaster_grants
                .push(BlasterGrantBatch {
                    drop_id,
                    player_id,
                    letter,
                    mag,
                    chamber,
                });
        }
        ServerToClient::Welcome { .. } | ServerToClient::Reject { .. } => {}
    }
}

#[derive(serde::Deserialize)]
struct WtIdentity {
    /// WebTransport listen port on the same host as the page.
    port: u16,
    hash_sha256: Vec<u8>,
}

async fn fetch_identity() -> Result<WtIdentity, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let resp_val = JsFuture::from(window.fetch_with_str(IDENTITY_PATH)).await?;
    let resp: web_sys::Response = resp_val.dyn_into()?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!(
            "identity fetch HTTP {}",
            resp.status()
        )));
    }
    let text = JsFuture::from(resp.text()?).await?;
    let text = text.as_string().ok_or_else(|| JsValue::from_str("text"))?;
    serde_json::from_str(&text).map_err(|e| JsValue::from_str(&format!("identity json: {e}")))
}

/// Same host as the Vite page; game-server listens on `identity.port`.
fn wt_url_from_page(identity: &WtIdentity) -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let host = window
        .location()
        .hostname()
        .map_err(|_| JsValue::from_str("location.hostname"))?;
    if host.is_empty() {
        return Err(JsValue::from_str("empty hostname"));
    }
    Ok(format!("https://{host}:{}/", identity.port))
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

pub fn js_error_string(e: &JsValue) -> String {
    if let Some(s) = e.as_string() {
        return s;
    }
    if let Some(obj) = e.dyn_ref::<js_sys::Object>() {
        if let Ok(msg) = Reflect::get(obj, &"message".into()) {
            if let Some(s) = msg.as_string() {
                return s;
            }
        }
    }
    format!("{e:?}")
}
