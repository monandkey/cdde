mod session;
mod store;
mod client;
mod network;
mod integration_test;

pub use session::TransactionContext;
pub use store::TransactionStore;
pub use client::DcrClient;
pub use network::TcpServer;

use cdde_logging;
use cdde_metrics;
use tracing::info;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize logging
    cdde_logging::init();
    
    // Register metrics
    cdde_metrics::register_metrics();
    
    info!(
        service = "dfl",
        version = env!("CARGO_PKG_VERSION"),
        "Starting Diameter Frontline service"
    );

    // Initialize DCR client
    let dcr_endpoint = std::env::var("DCR_ENDPOINT").unwrap_or_else(|_| "http://[::1]:50051".to_string());
    let _dcr_client = DcrClient::new(dcr_endpoint.clone());
    
    info!("Initialized DCR client pointing to {}", dcr_endpoint);

    // Initialize Session Store
    let store = Arc::new(TransactionStore::new());

    // Start TCP Server
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3868".to_string());
    let server = TcpServer::new(bind_addr.clone(), store);

    info!("Starting TCP listener on {}", bind_addr);
    
    if let Err(e) = server.start().await {
        info!("Server error: {}", e);
    }
}
