//! Native multiplayer authority host (022).
//!
//! Listens for binary WebSocket frames (postcard `ClientToServer` /
//! `ServerToClient`). Fixed tick advances with or without clients.

mod map;
mod world;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use game_net::TICK_HZ;
use game_net::{decode_c2s, encode_s2c, ClientToServer, ServerToClient, CONTENT_REV};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};
use world::{snapshot_msg, World};

/// Default listen address (separate from Vite :3000).
const DEFAULT_BIND: &str = "0.0.0.0:9090";

type OutTx = mpsc::UnboundedSender<Vec<u8>>;

struct ClientSlot {
    out: OutTx,
}

struct ServerState {
    world: World,
    clients: HashMap<game_net::PlayerId, ClientSlot>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            world: World::new(),
            clients: HashMap::new(),
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let bind = std::env::var("GAME_SERVER_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_string());
    let listener = TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("bind {bind}: {e}"));
    info!(
        %bind,
        tick_hz = TICK_HZ,
        content_rev = CONTENT_REV,
        "game-server listening"
    );

    let state = Arc::new(Mutex::new(ServerState::new()));

    {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            tick_loop(state).await;
        });
    }

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(state, stream, addr).await {
                        warn!(%addr, error = %e, "connection closed with error");
                    }
                });
            }
            Err(e) => warn!(error = %e, "accept failed"),
        }
    }
}

async fn tick_loop(state: Arc<Mutex<ServerState>>) {
    let dt = 1.0 / TICK_HZ as f32;
    let mut interval = tokio::time::interval(Duration::from_secs_f32(dt));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        let mut guard = state.lock().await;
        guard.world.advance_tick(dt);

        let ids: Vec<_> = guard.clients.keys().copied().collect();
        let mut sends: Vec<(OutTx, Vec<u8>)> = Vec::with_capacity(ids.len());
        for id in ids {
            let msg = snapshot_msg(&guard.world, id);
            match encode_s2c(&msg) {
                Ok(bytes) => {
                    if let Some(slot) = guard.clients.get(&id) {
                        sends.push((slot.out.clone(), bytes));
                    }
                }
                Err(e) => warn!(player = id, error = %e, "encode snapshot"),
            }
        }
        let tick = guard.world.tick;
        let n = guard.world.player_count();
        drop(guard);

        for (tx, bytes) in sends {
            let _ = tx.send(bytes);
        }

        if tick % (TICK_HZ * 5) == 0 {
            info!(tick, players = n, "tick");
        }
    }
}

async fn handle_connection(
    state: Arc<Mutex<ServerState>>,
    stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    info!(%addr, "websocket accepted");
    let (mut sink, mut stream) = ws.split();

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let mut player_id: Option<game_net::PlayerId> = None;

    let write = tokio::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            if sink.send(Message::Binary(bytes)).await.is_err() {
                break;
            }
        }
    });

    while let Some(frame) = stream.next().await {
        let frame = match frame {
            Ok(f) => f,
            Err(e) => {
                warn!(%addr, error = %e, "ws read");
                break;
            }
        };
        match frame {
            Message::Binary(bytes) => {
                let msg = match decode_c2s(&bytes) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!(%addr, error = %e, "decode c2s");
                        continue;
                    }
                };
                match msg {
                    ClientToServer::Hello(hello) => {
                        if player_id.is_some() {
                            continue;
                        }
                        let mut guard = state.lock().await;
                        match guard.world.try_join(&hello) {
                            Ok((id, welcome)) => {
                                player_id = Some(id);
                                guard.clients.insert(
                                    id,
                                    ClientSlot {
                                        out: out_tx.clone(),
                                    },
                                );
                                let bytes = encode_s2c(&ServerToClient::Welcome(welcome))?;
                                drop(guard);
                                let _ = out_tx.send(bytes);
                                info!(%addr, player = id, "joined");
                            }
                            Err(reject) => {
                                drop(guard);
                                let bytes = encode_s2c(&ServerToClient::Reject(reject))?;
                                let _ = out_tx.send(bytes);
                                info!(%addr, "rejected");
                            }
                        }
                    }
                    ClientToServer::Input(input) => {
                        let Some(id) = player_id else {
                            continue;
                        };
                        let mut guard = state.lock().await;
                        let _ = guard.world.queue_input(id, input);
                    }
                }
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) | Message::Text(_) => {}
        }
    }

    if let Some(id) = player_id {
        let mut guard = state.lock().await;
        guard.clients.remove(&id);
        if let Some(left) = guard.world.remove_player(id) {
            let msg = ServerToClient::PlayerLeft(left);
            if let Ok(bytes) = encode_s2c(&msg) {
                for slot in guard.clients.values() {
                    let _ = slot.out.send(bytes.clone());
                }
            }
        }
        info!(%addr, player = id, "left");
    }

    drop(out_tx);
    let _ = write.await;
    Ok(())
}
