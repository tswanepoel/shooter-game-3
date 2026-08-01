//! Per-connection WebTransport session.

use std::sync::Arc;

use game_net::{
    decode_c2s, display_name_key, drain_c2s_frames, encode_s2c, encode_s2c_frame,
    normalize_display_name, take_c2s_frame, ClientToServer, NetRole, PlayerId, ServerToClient,
    DEFAULT_CHARACTER, PROTOCOL_VERSION,
};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};
use wtransport::Connection;

use crate::clock::{IdAllocator, SharedClock};
use crate::roster::{normalize_room_code, spawn_pose, CombatState, PeerEntry, Rooms};

pub type SessionError = Box<dyn std::error::Error + Send + Sync>;

pub async fn handle_session(
    incoming: wtransport::endpoint::IncomingSession,
    clock: Arc<SharedClock>,
    ids: Arc<IdAllocator>,
    rooms: Arc<Mutex<Rooms>>,
) -> Result<(), SessionError> {
    let connection = accept_connection(incoming).await?;
    let (send, recv) = connection.accept_bi().await?;

    let Some(hello) = accept_hello(send, recv, &rooms).await? else {
        return Ok(());
    };
    let player_id = ids.alloc();
    let (reliable_tx, control_rx) =
        admit_member(&connection, hello, player_id, &clock, &rooms).await?;

    session_loop(
        &connection,
        player_id,
        clock,
        rooms,
        control_rx,
        reliable_tx,
    )
    .await
}

async fn accept_connection(
    incoming: wtransport::endpoint::IncomingSession,
) -> Result<Connection, SessionError> {
    let request = incoming.await?;
    info!(
        wt_host = %request.authority(),
        path = %request.path(),
        "session request"
    );
    let connection = request.accept().await?;
    info!("session accepted");
    Ok(connection)
}

struct HelloOk {
    room_code: String,
    display_name: String,
    send: wtransport::SendStream,
    recv: wtransport::RecvStream,
    /// Bytes the client coalesced behind Hello; belongs to the control reader.
    carry: Vec<u8>,
}

async fn write_reject(
    send: &mut wtransport::SendStream,
    reason: impl Into<String>,
) -> Result<(), SessionError> {
    let rej = encode_s2c_frame(&ServerToClient::Reject {
        reason: reason.into(),
    })?;
    send.write_all(&rej).await?;
    Ok(())
}

/// Validate Hello. Returns `None` when the session was rejected cleanly.
async fn accept_hello(
    mut send: wtransport::SendStream,
    mut recv: wtransport::RecvStream,
    rooms: &Mutex<Rooms>,
) -> Result<Option<HelloOk>, SessionError> {
    let mut carry = Vec::new();
    let mut chunk = vec![0u8; 4096];
    let first = loop {
        if let Some(msg) = take_c2s_frame(&mut carry)? {
            break msg;
        }
        let n = recv
            .read(&mut chunk)
            .await?
            .ok_or("client closed stream before Hello")?;
        carry.extend_from_slice(&chunk[..n]);
    };

    let (protocol, room_code_raw, display_name_raw) = match first {
        ClientToServer::Hello {
            protocol,
            room_code,
            display_name,
        } => (protocol, room_code, display_name),
        _ => {
            write_reject(&mut send, "expected Hello").await?;
            return Ok(None);
        }
    };

    if protocol != PROTOCOL_VERSION {
        write_reject(
            &mut send,
            format!("protocol mismatch: got {protocol}, want {PROTOCOL_VERSION}"),
        )
        .await?;
        return Ok(None);
    }

    let room_code = match normalize_room_code(&room_code_raw) {
        Ok(c) => c,
        Err(reason) => {
            write_reject(&mut send, reason).await?;
            return Ok(None);
        }
    };

    let display_name = match normalize_display_name(&display_name_raw) {
        Ok(n) => n,
        Err(reason) => {
            write_reject(&mut send, reason).await?;
            return Ok(None);
        }
    };
    let name_key = display_name_key(&display_name);

    {
        let guard = rooms.lock().await;
        if guard.name_taken_in(&room_code, &name_key) {
            drop(guard);
            write_reject(&mut send, "display name taken").await?;
            return Ok(None);
        }
    }

    Ok(Some(HelloOk {
        room_code,
        display_name,
        send,
        recv,
        carry,
    }))
}

async fn admit_member(
    connection: &Connection,
    hello: HelloOk,
    player_id: PlayerId,
    clock: &SharedClock,
    rooms: &Mutex<Rooms>,
) -> Result<
    (
        mpsc::UnboundedSender<Vec<u8>>,
        mpsc::UnboundedReceiver<ClientToServer>,
    ),
    SessionError,
> {
    let HelloOk {
        room_code,
        display_name,
        mut send,
        mut recv,
        mut carry,
    } = hello;

    let welcome = encode_s2c_frame(&ServerToClient::Welcome {
        protocol: PROTOCOL_VERSION,
        player_id,
        tick: clock.tick(),
        server_time_secs: clock.server_time_secs(),
    })?;
    send.write_all(&welcome).await?;
    info!(player_id, %room_code, %display_name, "welcomed");

    let (reliable_tx, mut reliable_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::spawn(async move {
        while let Some(bytes) = reliable_rx.recv().await {
            if send.write_all(&bytes).await.is_err() {
                break;
            }
        }
    });

    let (control_tx, control_rx) = mpsc::unbounded_channel::<ClientToServer>();
    tokio::spawn(async move {
        let mut chunk = vec![0u8; 4096];
        loop {
            let msgs = match drain_c2s_frames(&mut carry) {
                Ok(msgs) => msgs,
                Err(e) => {
                    warn!(player_id, "control stream: {e}");
                    break;
                }
            };
            if msgs.into_iter().any(|msg| control_tx.send(msg).is_err()) {
                break;
            }
            match recv.read(&mut chunk).await {
                Ok(Some(n)) => carry.extend_from_slice(&chunk[..n]),
                _ => break,
            }
        }
    });

    {
        let mut guard = rooms.lock().await;
        let join_tick = clock.tick();
        guard.insert(
            room_code,
            player_id,
            PeerEntry {
                connection: connection.clone(),
                reliable_tx: reliable_tx.clone(),
                display_name,
                combat: CombatState::fresh(),
                role: NetRole::Player,
                character: DEFAULT_CHARACTER,
                last_drive: None,
            },
        );
        guard.broadcast_roster(player_id, join_tick);
    }

    Ok((reliable_tx, control_rx))
}

async fn session_loop(
    connection: &Connection,
    player_id: PlayerId,
    clock: Arc<SharedClock>,
    rooms: Arc<Mutex<Rooms>>,
    mut control_rx: mpsc::UnboundedReceiver<ClientToServer>,
    _reliable_tx: mpsc::UnboundedSender<Vec<u8>>,
) -> Result<(), SessionError> {
    loop {
        tokio::select! {
            err = connection.closed() => {
                info!(player_id, "connection closed: {err}");
                break;
            }
            control = control_rx.recv() => {
                match control {
                    Some(msg) => {
                        handle_control_msg(player_id, msg, &clock, &rooms).await;
                    }
                    None => {
                        info!(player_id, "reliable control closed");
                        break;
                    }
                }
            }
            dgram = connection.receive_datagram() => {
                match dgram {
                    Ok(dgram) => {
                        let t2 = clock.server_time_secs();
                        if let Ok(msg) = decode_c2s(dgram.payload().as_ref()) {
                            if handle_datagram(
                                player_id,
                                msg,
                                t2,
                                connection,
                                &clock,
                                &rooms,
                            )
                            .await
                            {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("receive_datagram: {e}");
                        break;
                    }
                }
            }
        }
    }

    remove_member(player_id, &clock, &rooms).await;
    Ok(())
}

async fn handle_control_msg(
    player_id: PlayerId,
    msg: ClientToServer,
    clock: &SharedClock,
    rooms: &Mutex<Rooms>,
) {
    match msg {
        ClientToServer::Spawn {
            primary,
            secondary,
            active: _,
        } => handle_spawn(player_id, primary, secondary, clock, rooms).await,
        ClientToServer::SetRole { role } => handle_set_role(player_id, role, clock, rooms).await,
        ClientToServer::SetCharacter { character } => {
            handle_set_character(player_id, character, clock, rooms).await;
        }
        ClientToServer::PickMap { map } => {
            handle_pick_map(player_id, map, clock, rooms).await;
        }
        ClientToServer::StartMatch => handle_start_match(player_id, clock, rooms).await,
        _ => {}
    }
}

/// `true` = tear down session.
async fn handle_datagram(
    player_id: PlayerId,
    msg: ClientToServer,
    t2: f64,
    connection: &Connection,
    clock: &SharedClock,
    rooms: &Mutex<Rooms>,
) -> bool {
    match msg {
        ClientToServer::ClockProbe { t1 } => {
            let t3 = clock.server_time_secs();
            let Ok(reply) = encode_s2c(&ServerToClient::ClockReply {
                t1,
                t2,
                t3,
                tick: clock.tick(),
            }) else {
                return false;
            };
            if let Err(e) = connection.send_datagram(reply) {
                warn!("send_datagram: {e}");
                return true;
            }
            false
        }
        ClientToServer::DriveSample { tick, mut drive } => {
            let guard = rooms.lock().await;
            if !guard.living(player_id) {
                return false;
            }
            // Authority: roster kit wins over client-claimed drive.character.
            if let Some(ch) = guard.character(player_id) {
                drive.character = ch;
            }
            let Ok(relay) = encode_s2c(&ServerToClient::PeerDrive {
                tick,
                id: player_id,
                drive: drive.clone(),
            }) else {
                return false;
            };
            drop(guard);
            {
                let mut guard = rooms.lock().await;
                guard.note_drive(player_id, drive);
                guard.relay_datagram(player_id, &relay);
            }
            false
        }
        ClientToServer::ProjectileSpawn { tick, projectiles } => {
            if projectiles.is_empty() {
                return false;
            }
            let guard = rooms.lock().await;
            if !guard.living(player_id) {
                return false;
            }
            let Ok(relay) = encode_s2c(&ServerToClient::PeerProjectileSpawn {
                tick,
                id: player_id,
                projectiles,
            }) else {
                return false;
            };
            guard.relay_datagram(player_id, &relay);
            false
        }
        ClientToServer::ImpactHit { tick, hit } => {
            let victim = hit.target;
            let outcome = {
                let mut guard = rooms.lock().await;
                guard.apply_impact(player_id, &hit)
            };
            match outcome {
                Err(reason) => {
                    warn!(
                        firer = player_id,
                        victim,
                        part = hit.part,
                        ammo = hit.ammo,
                        speed = hit.speed,
                        "impact claim rejected: {reason}"
                    );
                    // 080: never relay rejects — peers must not present unaccepted damage.
                }
                Ok(false) => {
                    let Ok(relay) = encode_s2c(&ServerToClient::PeerImpactHit {
                        tick,
                        id: player_id,
                        hit,
                    }) else {
                        return false;
                    };
                    let guard = rooms.lock().await;
                    guard.relay_datagram(player_id, &relay);
                }
                Ok(true) => {
                    info!(firer = player_id, victim, "kill accepted");
                    let t = clock.tick();
                    let Ok(death) = encode_s2c_frame(&ServerToClient::DeathAnnounce {
                        tick: t,
                        victim,
                        killer: player_id,
                    }) else {
                        return false;
                    };
                    {
                        let mut guard = rooms.lock().await;
                        guard.broadcast_reliable_all(player_id, &death);
                        guard.spawn_corpse_for_kill(victim, t);
                        guard.broadcast_roster(player_id, t);
                    }
                }
            }
            false
        }
        ClientToServer::AmmoDump { tick, dump } => {
            let mut guard = rooms.lock().await;
            guard.accept_ammo_dump(player_id, dump.ammo, dump.rounds, dump.position, tick);
            false
        }
        ClientToServer::LootClaim {
            tick,
            drop_id,
            position,
            room,
        } => {
            let mut guard = rooms.lock().await;
            guard.accept_loot_claim(player_id, drop_id, position, room, tick);
            false
        }
        ClientToServer::BlasterDump { tick, dump } => {
            let mut guard = rooms.lock().await;
            guard.accept_blaster_dump(player_id, dump.letter, dump.mag, dump.position, tick);
            false
        }
        ClientToServer::BlasterClaim {
            tick,
            drop_id,
            position,
        } => {
            let mut guard = rooms.lock().await;
            guard.accept_blaster_claim(player_id, drop_id, position, tick);
            false
        }
        // Reliable-stream only.
        ClientToServer::Spawn { .. }
        | ClientToServer::SetRole { .. }
        | ClientToServer::SetCharacter { .. }
        | ClientToServer::PickMap { .. }
        | ClientToServer::StartMatch
        | ClientToServer::Hello { .. } => false,
    }
}

async fn handle_set_role(
    player_id: PlayerId,
    role: NetRole,
    clock: &SharedClock,
    rooms: &Mutex<Rooms>,
) {
    let tick = clock.tick();
    let mut guard = rooms.lock().await;
    if guard.set_role(player_id, role) {
        guard.broadcast_roster(player_id, tick);
    }
}

async fn handle_set_character(
    player_id: PlayerId,
    character: u8,
    clock: &SharedClock,
    rooms: &Mutex<Rooms>,
) {
    let tick = clock.tick();
    let mut guard = rooms.lock().await;
    if guard.set_character(player_id, character) {
        guard.broadcast_roster(player_id, tick);
    }
}

async fn handle_pick_map(player_id: PlayerId, map: u8, clock: &SharedClock, rooms: &Mutex<Rooms>) {
    let tick = clock.tick();
    let mut guard = rooms.lock().await;
    if guard.try_pick_map(player_id, map) {
        guard.broadcast_roster(player_id, tick);
    }
}

async fn handle_start_match(player_id: PlayerId, clock: &SharedClock, rooms: &Mutex<Rooms>) {
    let tick = clock.tick();
    let mut guard = rooms.lock().await;
    if guard.try_start_match(player_id) {
        guard.broadcast_roster(player_id, tick);
    }
}

async fn handle_spawn(
    player_id: PlayerId,
    primary: Option<u8>,
    secondary: Option<u8>,
    clock: &SharedClock,
    rooms: &Mutex<Rooms>,
) {
    let tick = clock.tick();
    let (position, facing) = spawn_pose(tick, player_id);
    let granted = {
        let mut guard = rooms.lock().await;
        guard.try_spawn(player_id, primary, secondary)
    };
    if let Err(reason) = granted {
        // The client resends Spawn until YouSpawned arrives, so a repeating
        // reject here is a client stuck on the bench.
        warn!(player_id, "spawn rejected: {reason}");
        return;
    }
    info!(player_id, tick, "spawn granted");
    let Ok(you) = encode_s2c_frame(&ServerToClient::YouSpawned {
        tick,
        position,
        facing,
    }) else {
        return;
    };
    let guard = rooms.lock().await;
    guard.send_reliable(player_id, you);
    guard.broadcast_roster(player_id, tick);
}

async fn remove_member(player_id: PlayerId, clock: &SharedClock, rooms: &Mutex<Rooms>) {
    let mut guard = rooms.lock().await;
    let tick = clock.tick();
    if guard.leave(player_id, tick) {
        info!(player_id, "peer left");
    }
}
