use cdde_dpa_core::types::PeerConfig;
use cdde_dpa_runtime::peer_actor::PeerActor;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    cdde_logging::init();

    info!(
        service = "dpa",
        version = env!("CARGO_PKG_VERSION"),
        "Starting Diameter Peer Agent service"
    );

    // DFL通知用チャネル
    let (dfl_tx, mut dfl_rx) = tokio::sync::mpsc::channel(100);

    // DFL通知受信ループ (簡易実装)
    tokio::spawn(async move {
        while let Some(msg) = dfl_rx.recv().await {
            info!("Received from PeerActor: {}", msg);
        }
    });

    // Peer Actorの起動 (例: 1つのピア)
    let peer_config = PeerConfig {
        watchdog_interval: Duration::from_secs(30),
        max_watchdog_failures: 3,
    };

    let peer_addr = std::env::var("PEER_ADDR").unwrap_or_else(|_| "127.0.0.1:3868".to_string());
    
    let mut actor = PeerActor::new(peer_addr.clone(), peer_config, dfl_tx);
    
    info!("Starting PeerActor for {}", peer_addr);
    actor.run().await;
}
