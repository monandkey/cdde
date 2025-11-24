mod state_machine;
mod connector;

pub use state_machine::PeerStateMachine;
pub use connector::TcpClient;

use cdde_logging;
use cdde_metrics;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    cdde_logging::init();
    
    // Register metrics
    cdde_metrics::register_metrics();
    
    info!(
        service = "dpa",
        version = env!("CARGO_PKG_VERSION"),
        "Starting Diameter Peer Agent service"
    );

    // Initialize State Machine
    let peer_addr = std::env::var("PEER_ADDR").unwrap_or_else(|_| "127.0.0.1:3868".to_string());
    let _fsm = PeerStateMachine::new(peer_addr.clone());

    // Start Connector
    let client = TcpClient::new(peer_addr);
    
    // Spawn client loop
    tokio::spawn(async move {
        client.start().await;
    });

    // Keep main alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
