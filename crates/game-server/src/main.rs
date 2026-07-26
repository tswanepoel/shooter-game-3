//! Native multiplayer host (WebTransport).

mod clock;
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
    let rooms = Arc::new(Mutex::new(Rooms::new()));

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
        let rooms = Arc::clone(&rooms);
        tokio::spawn(async move {
            if let Err(e) = handle_session(incoming, clock, ids, rooms).await {
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
