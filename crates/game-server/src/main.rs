//! Native multiplayer host (WebTransport).

mod clock;
mod loot;
mod roster;
mod session;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use game_net::{PROTOCOL_VERSION, TICK_DURATION_SECS, TICK_HZ};
use tokio::sync::Mutex;
use tracing::{info, warn};
use wtransport::tls::Sha256DigestFmt;
use wtransport::{Endpoint, Identity, ServerConfig};

use clock::{IdAllocator, SharedClock};
use roster::Rooms;
use session::handle_session;

const DEFAULT_BIND: &str = "0.0.0.0:4433";

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

    // Clients dial the page hostname on this port (same machine as Vite). Cert SANs
    // must cover that host — auto-detect LAN IPs; optional GAME_SERVER_PUBLIC_HOST adds more.
    let sans = cert_sans();
    let identity =
        Identity::self_signed(sans.iter().map(String::as_str)).expect("self-signed identity");
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
    let rooms = Arc::new(Mutex::new(Rooms::new()));

    {
        let clock = Arc::clone(&clock);
        let rooms = Arc::clone(&rooms);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs_f64(TICK_DURATION_SECS));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval.tick().await;
            loop {
                interval.tick().await;
                clock.advance();
                let mut guard = rooms.lock().await;
                guard.tick_loot(TICK_DURATION_SECS as f32, clock.tick());
            }
        });
    }

    info!(
        %bind,
        ?sans,
        tick_hz = TICK_HZ,
        protocol = PROTOCOL_VERSION,
        cert = %cert_hash.fmt(Sha256DigestFmt::DottedHex),
        "game-server listening (WebTransport)"
    );

    loop {
        let incoming = server.accept().await;
        let clock = Arc::clone(&clock);
        let ids = Arc::clone(&ids);
        let rooms = Arc::clone(&rooms);
        tokio::spawn(async move {
            if let Err(e) = handle_session(incoming, clock, ids, rooms).await {
                warn!("session ended: {e}");
            }
        });
    }
}

fn cert_sans() -> Vec<String> {
    let mut sans = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];
    for host in local_lan_hosts() {
        push_unique(&mut sans, host);
    }
    if let Ok(extra) = std::env::var("GAME_SERVER_PUBLIC_HOST") {
        for host in extra.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            push_unique(&mut sans, host.to_string());
        }
    }
    sans
}

fn local_lan_hosts() -> Vec<String> {
    let mut hosts = Vec::new();
    // UDP connect fills the local address of the primary outbound NIC (no packets sent).
    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if sock.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = sock.local_addr() {
                let ip = addr.ip();
                if !ip.is_loopback() && !ip.is_unspecified() {
                    hosts.push(ip.to_string());
                }
            }
        }
    }
    hosts
}

fn push_unique(sans: &mut Vec<String>, host: String) {
    if !sans.iter().any(|s| s == &host) {
        sans.push(host);
    }
}

fn write_identity_file(port: u16, hash: &[u8; 32]) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../debug/wt-identity.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Clients build `https://{page-hostname}:{port}/` themselves — only hash + port here.
    let doc = serde_json::json!({
        "port": port,
        "hash_sha256": hash.as_slice(),
    });
    match std::fs::write(&path, serde_json::to_vec_pretty(&doc).unwrap()) {
        Ok(()) => info!(?path, "wrote WebTransport identity for dev clients"),
        Err(e) => warn!(?path, "failed to write identity file: {e}"),
    }
}
