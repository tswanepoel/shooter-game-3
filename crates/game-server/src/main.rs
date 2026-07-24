//! Native multiplayer host (WebTransport).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use game_net::{
    decode_c2s, encode_s2c, encode_s2c_frame, ClientToServer, PlayerId, ServerToClient,
    PROTOCOL_VERSION, TICK_DURATION_SECS, TICK_HZ,
};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};
use wtransport::tls::Sha256DigestFmt;
use wtransport::{Connection, Endpoint, Identity, ServerConfig};

const DEFAULT_BIND: &str = "0.0.0.0:4433";

struct SharedClock {
    epoch: Instant,
    tick: AtomicU64,
}

impl SharedClock {
    fn new() -> Self {
        Self {
            epoch: Instant::now(),
            tick: AtomicU64::new(0),
        }
    }

    fn server_time_secs(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    fn tick(&self) -> u64 {
        self.tick.load(Ordering::Relaxed)
    }

    fn advance(&self) {
        self.tick.fetch_add(1, Ordering::Relaxed);
    }
}

struct IdAllocator {
    next: AtomicU32,
}

impl IdAllocator {
    fn new() -> Self {
        Self {
            next: AtomicU32::new(1),
        }
    }

    fn alloc(&self) -> PlayerId {
        self.next.fetch_add(1, Ordering::Relaxed)
    }
}

struct PeerEntry {
    connection: Connection,
    reliable_tx: mpsc::UnboundedSender<Vec<u8>>,
}

struct Roster {
    peers: HashMap<PlayerId, PeerEntry>,
}

impl Roster {
    fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    fn ids(&self) -> Vec<PlayerId> {
        self.peers.keys().copied().collect()
    }

    fn insert(&mut self, id: PlayerId, entry: PeerEntry) {
        self.peers.insert(id, entry);
    }

    fn remove(&mut self, id: PlayerId) -> bool {
        self.peers.remove(&id).is_some()
    }

    fn contains(&self, id: PlayerId) -> bool {
        self.peers.contains_key(&id)
    }

    fn broadcast_reliable(&self, except: Option<PlayerId>, bytes: &[u8]) {
        for (&id, peer) in &self.peers {
            if Some(id) == except {
                continue;
            }
            let _ = peer.reliable_tx.send(bytes.to_vec());
        }
    }

    fn relay_datagram(&self, except: PlayerId, bytes: &[u8]) {
        for (&id, peer) in &self.peers {
            if id == except {
                continue;
            }
            if let Err(e) = peer.connection.send_datagram(bytes) {
                warn!(peer = id, "send_datagram: {e}");
            }
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

    let bind: SocketAddr = std::env::var("GAME_SERVER_BIND")
        .unwrap_or_else(|_| DEFAULT_BIND.into())
        .parse()
        .expect("GAME_SERVER_BIND must be host:port");

    let identity =
        Identity::self_signed(["localhost", "127.0.0.1", "::1"]).expect("self-signed identity");
    let cert_hash = identity.certificate_chain().as_slice()[0].hash();
    let hash_bytes = *cert_hash.as_ref();

    write_identity_file(bind.port(), &hash_bytes);

    let config = ServerConfig::builder()
        .with_bind_address(bind)
        .with_identity(identity)
        .keep_alive_interval(Some(Duration::from_secs(3)))
        .build();

    let server = Endpoint::server(config).expect("WebTransport endpoint");
    let clock = Arc::new(SharedClock::new());
    let ids = Arc::new(IdAllocator::new());
    let roster = Arc::new(Mutex::new(Roster::new()));

    {
        let clock = Arc::clone(&clock);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs_f64(TICK_DURATION_SECS));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval.tick().await;
            loop {
                interval.tick().await;
                clock.advance();
            }
        });
    }

    info!(
        %bind,
        tick_hz = TICK_HZ,
        protocol = PROTOCOL_VERSION,
        cert = %cert_hash.fmt(Sha256DigestFmt::DottedHex),
        "game-server listening (WebTransport)"
    );

    loop {
        let incoming = server.accept().await;
        let clock = Arc::clone(&clock);
        let ids = Arc::clone(&ids);
        let roster = Arc::clone(&roster);
        tokio::spawn(async move {
            if let Err(e) = handle_session(incoming, clock, ids, roster).await {
                warn!("session ended: {e}");
            }
        });
    }
}

fn write_identity_file(port: u16, hash: &[u8; 32]) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../debug/wt-identity.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let doc = serde_json::json!({
        "url": format!("https://127.0.0.1:{port}/"),
        "hash_sha256": hash.as_slice(),
    });
    match std::fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()) {
        Ok(()) => info!(?path, "wrote WebTransport identity for dev clients"),
        Err(e) => warn!(?path, "failed to write identity file: {e}"),
    }
}

async fn handle_session(
    incoming: wtransport::endpoint::IncomingSession,
    clock: Arc<SharedClock>,
    ids: Arc<IdAllocator>,
    roster: Arc<Mutex<Roster>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request = incoming.await?;
    info!(
        wt_host = %request.authority(),
        path = %request.path(),
        "session request"
    );
    let connection = request.accept().await?;
    info!("session accepted");

    let (mut send, mut recv) = connection.accept_bi().await?;
    let mut buf = vec![0u8; 1024];
    let n = recv
        .read(&mut buf)
        .await?
        .ok_or("client closed stream before Hello")?;

    let hello = match decode_c2s(&buf[..n]) {
        Ok(ClientToServer::Hello { protocol }) => protocol,
        Ok(_) => {
            let rej = encode_s2c(&ServerToClient::Reject {
                reason: "expected Hello".into(),
            })?;
            send.write_all(&rej).await?;
            return Ok(());
        }
        Err(_) => {
            let rej = encode_s2c(&ServerToClient::Reject {
                reason: "expected Hello".into(),
            })?;
            send.write_all(&rej).await?;
            return Ok(());
        }
    };

    if hello != PROTOCOL_VERSION {
        let rej = encode_s2c(&ServerToClient::Reject {
            reason: format!("protocol mismatch: got {hello}, want {PROTOCOL_VERSION}"),
        })?;
        send.write_all(&rej).await?;
        return Ok(());
    }

    let player_id = ids.alloc();
    let welcome = encode_s2c(&ServerToClient::Welcome {
        protocol: PROTOCOL_VERSION,
        player_id,
        tick: clock.tick(),
        server_time_secs: clock.server_time_secs(),
    })?;
    send.write_all(&welcome).await?;
    info!(player_id, "welcomed");

    let (reliable_tx, mut reliable_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    {
        let mut send = send;
        tokio::spawn(async move {
            while let Some(bytes) = reliable_rx.recv().await {
                if send.write_all(&bytes).await.is_err() {
                    break;
                }
            }
        });
    }

    {
        let mut guard = roster.lock().await;
        let existing: Vec<PlayerId> = guard.ids();
        let join_tick = clock.tick();
        let join_msg = encode_s2c_frame(&ServerToClient::PeerJoined {
            tick: join_tick,
            id: player_id,
        })?;
        guard.broadcast_reliable(None, &join_msg);

        for other_id in existing {
            let msg = encode_s2c_frame(&ServerToClient::PeerJoined {
                tick: join_tick,
                id: other_id,
            })?;
            let _ = reliable_tx.send(msg);
        }

        guard.insert(
            player_id,
            PeerEntry {
                connection: connection.clone(),
                reliable_tx: reliable_tx.clone(),
            },
        );
    }

    // Keep a sender clone so the reliable writer stays open until leave.
    let _reliable_tx = reliable_tx;

    loop {
        tokio::select! {
            err = connection.closed() => {
                info!(player_id, "connection closed: {err}");
                break;
            }
            dgram = connection.receive_datagram() => {
                match dgram {
                    Ok(dgram) => {
                        let t2 = clock.server_time_secs();
                        match decode_c2s(dgram.payload().as_ref()) {
                            Ok(ClientToServer::ClockProbe { t1 }) => {
                                let t3 = clock.server_time_secs();
                                let reply = encode_s2c(&ServerToClient::ClockReply {
                                    t1,
                                    t2,
                                    t3,
                                    tick: clock.tick(),
                                })?;
                                if let Err(e) = connection.send_datagram(reply) {
                                    warn!("send_datagram: {e}");
                                    break;
                                }
                            }
                            Ok(ClientToServer::DriveSample { tick, drive }) => {
                                let joined = {
                                    let guard = roster.lock().await;
                                    guard.contains(player_id)
                                };
                                if !joined {
                                    continue;
                                }
                                let relay = encode_s2c(&ServerToClient::PeerDrive {
                                    tick,
                                    id: player_id,
                                    drive,
                                })?;
                                let guard = roster.lock().await;
                                guard.relay_datagram(player_id, &relay);
                            }
                            Ok(ClientToServer::ProjectileSpawn { tick, projectiles }) => {
                                let joined = {
                                    let guard = roster.lock().await;
                                    guard.contains(player_id)
                                };
                                if !joined || projectiles.is_empty() {
                                    continue;
                                }
                                // Claim-and-relay only; server does not own projectiles (038).
                                let relay = encode_s2c(&ServerToClient::PeerProjectileSpawn {
                                    tick,
                                    id: player_id,
                                    projectiles,
                                })?;
                                let guard = roster.lock().await;
                                guard.relay_datagram(player_id, &relay);
                            }
                            Ok(ClientToServer::ImpactHit { tick, hit }) => {
                                let joined = {
                                    let guard = roster.lock().await;
                                    guard.contains(player_id)
                                };
                                if !joined {
                                    continue;
                                }
                                // Claim-and-relay; server does not re-sim the shot (043).
                                // Not echoed to firer → each present applies once.
                                let relay = encode_s2c(&ServerToClient::PeerImpactHit {
                                    tick,
                                    id: player_id,
                                    hit,
                                })?;
                                let guard = roster.lock().await;
                                guard.relay_datagram(player_id, &relay);
                            }
                            Ok(ClientToServer::Hello { .. }) => {}
                            Err(_) => {}
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

    {
        let mut guard = roster.lock().await;
        if guard.remove(player_id) {
            if let Ok(msg) = encode_s2c_frame(&ServerToClient::PeerLeft {
                tick: clock.tick(),
                id: player_id,
            }) {
                guard.broadcast_reliable(None, &msg);
            }
            info!(player_id, "peer left, roster notified");
        }
    }

    Ok(())
}
