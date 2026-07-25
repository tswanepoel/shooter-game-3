//! Per-connection WebTransport session.

use std::sync::Arc;

use game_net::{
    decode_c2s, display_name_key, encode_s2c, encode_s2c_frame, normalize_display_name,
    ClientToServer, NetRole, PlayerId, ServerToClient, DEFAULT_CHARACTER, DEFAULT_ROOM_CODE,
    PROTOCOL_VERSION,
};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};
use wtransport::Connection;

use crate::clock::{IdAllocator, SharedClock};
use crate::roster::{spawn_pose, CombatState, PeerEntry, Roster};

pub type SessionError = Box<dyn std::error::Error + Send + Sync>;

pub async fn handle_session(
    incoming: wtransport::endpoint::IncomingSession,
    clock: Arc<SharedClock>,
    ids: Arc<IdAllocator>,
    roster: Arc<Mutex<Roster>>,
) -> Result<(), SessionError> {
    let connection = accept_connection(incoming).await?;
    let (send, recv) = connection.accept_bi().await?;

    let Some(hello) = accept_hello(send, recv, &roster).await? else {
        return Ok(());
    };
    let player_id = ids.alloc();
    let (reliable_tx, control_rx) =
        admit_member(&connection, hello, player_id, &clock, &roster).await?;

    session_loop(
        &connection,
        player_id,
        clock,
        roster,
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
    display_name: String,
    send: wtransport::SendStream,
    recv: wtransport::RecvStream,
}

async fn write_reject(
    send: &mut wtransport::SendStream,
    reason: impl Into<String>,
) -> Result<(), SessionError> {
    let rej = encode_s2c(&ServerToClient::Reject {
        reason: reason.into(),
    })?;
    send.write_all(&rej).await?;
    Ok(())
}

/// Validate Hello. Returns `None` when the session was rejected cleanly.
async fn accept_hello(
    mut send: wtransport::SendStream,
    mut recv: wtransport::RecvStream,
    roster: &Mutex<Roster>,
) -> Result<Option<HelloOk>, SessionError> {
    let mut buf = vec![0u8; 4096];
    let n = recv
        .read(&mut buf)
        .await?
        .ok_or("client closed stream before Hello")?;

    let (protocol, room_code, display_name_raw) = match decode_c2s(&buf[..n]) {
        Ok(ClientToServer::Hello {
            protocol,
            room_code,
            display_name,
        }) => (protocol, room_code, display_name),
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

    if room_code != DEFAULT_ROOM_CODE {
        write_reject(&mut send, format!("unknown room code: {room_code}")).await?;
        return Ok(None);
    }

    let display_name = match normalize_display_name(&display_name_raw) {
        Ok(n) => n,
        Err(reason) => {
            write_reject(&mut send, reason).await?;
            return Ok(None);
        }
    };
    let name_key = display_name_key(&display_name);

    {
        let guard = roster.lock().await;
        if guard.name_taken(&name_key) {
            drop(guard);
            write_reject(&mut send, "display name taken").await?;
            return Ok(None);
        }
    }

    Ok(Some(HelloOk {
        display_name,
        send,
        recv,
    }))
}

async fn admit_member(
    connection: &Connection,
    hello: HelloOk,
    player_id: PlayerId,
    clock: &SharedClock,
    roster: &Mutex<Roster>,
) -> Result<
    (
        mpsc::UnboundedSender<Vec<u8>>,
        mpsc::UnboundedReceiver<ClientToServer>,
    ),
    SessionError,
> {
    let HelloOk {
        display_name,
        mut send,
        mut recv,
    } = hello;

    let welcome = encode_s2c(&ServerToClient::Welcome {
        protocol: PROTOCOL_VERSION,
        player_id,
        tick: clock.tick(),
        server_time_secs: clock.server_time_secs(),
    })?;
    send.write_all(&welcome).await?;
    info!(player_id, %display_name, "welcomed");

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
        let mut buf = vec![0u8; 4096];
        while let Ok(Some(n)) = recv.read(&mut buf).await {
            if let Ok(msg) = decode_c2s(&buf[..n]) {
                if control_tx.send(msg).is_err() {
                    break;
                }
            }
        }
    });

    {
        let mut guard = roster.lock().await;
        let join_tick = clock.tick();
        guard.insert(
            player_id,
            PeerEntry {
                connection: connection.clone(),
                reliable_tx: reliable_tx.clone(),
                display_name,
                combat: CombatState::fresh(),
                role: NetRole::Player,
                character: DEFAULT_CHARACTER,
            },
        );
        guard.broadcast_roster(join_tick);
    }

    Ok((reliable_tx, control_rx))
}

async fn session_loop(
    connection: &Connection,
    player_id: PlayerId,
    clock: Arc<SharedClock>,
    roster: Arc<Mutex<Roster>>,
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
                        handle_control_msg(player_id, msg, &clock, &roster).await;
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
                                &roster,
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

    remove_member(player_id, &clock, &roster).await;
    Ok(())
}

async fn handle_control_msg(
    player_id: PlayerId,
    msg: ClientToServer,
    clock: &SharedClock,
    roster: &Mutex<Roster>,
) {
    match msg {
        ClientToServer::Spawn => handle_spawn(player_id, clock, roster).await,
        ClientToServer::SetRole { role } => handle_set_role(player_id, role, clock, roster).await,
        ClientToServer::SetCharacter { character } => {
            handle_set_character(player_id, character, clock, roster).await;
        }
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
    roster: &Mutex<Roster>,
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
            let guard = roster.lock().await;
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
                drive,
            }) else {
                return false;
            };
            guard.relay_datagram(player_id, &relay);
            false
        }
        ClientToServer::ProjectileSpawn { tick, projectiles } => {
            if projectiles.is_empty() {
                return false;
            }
            let guard = roster.lock().await;
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
            let roster_dirty = {
                let mut guard = roster.lock().await;
                guard.apply_impact(player_id, &hit)
            };
            let Ok(relay) = encode_s2c(&ServerToClient::PeerImpactHit {
                tick,
                id: player_id,
                hit,
            }) else {
                return false;
            };
            {
                let guard = roster.lock().await;
                guard.relay_datagram(player_id, &relay);
                if roster_dirty {
                    guard.broadcast_roster(clock.tick());
                }
            }
            false
        }
        // Reliable-stream only.
        ClientToServer::Spawn
        | ClientToServer::SetRole { .. }
        | ClientToServer::SetCharacter { .. }
        | ClientToServer::Hello { .. } => false,
    }
}

async fn handle_set_role(
    player_id: PlayerId,
    role: NetRole,
    clock: &SharedClock,
    roster: &Mutex<Roster>,
) {
    let tick = clock.tick();
    let mut guard = roster.lock().await;
    if guard.set_role(player_id, role) {
        guard.broadcast_roster(tick);
    }
}

async fn handle_set_character(
    player_id: PlayerId,
    character: u8,
    clock: &SharedClock,
    roster: &Mutex<Roster>,
) {
    let tick = clock.tick();
    let mut guard = roster.lock().await;
    if guard.set_character(player_id, character) {
        guard.broadcast_roster(tick);
    }
}

async fn handle_spawn(player_id: PlayerId, clock: &SharedClock, roster: &Mutex<Roster>) {
    let tick = clock.tick();
    let (position, yaw) = spawn_pose(tick, player_id);
    let ok = {
        let mut guard = roster.lock().await;
        guard.try_spawn(player_id)
    };
    if !ok {
        return;
    }
    let Ok(you) = encode_s2c_frame(&ServerToClient::YouSpawned {
        tick,
        position,
        yaw,
    }) else {
        return;
    };
    let guard = roster.lock().await;
    guard.send_reliable(player_id, you);
    guard.broadcast_roster(tick);
}

async fn remove_member(player_id: PlayerId, clock: &SharedClock, roster: &Mutex<Roster>) {
    let mut guard = roster.lock().await;
    if guard.remove(player_id) {
        let tick = clock.tick();
        guard.broadcast_roster(tick);
        info!(player_id, "peer left, roster notified");
    }
}
