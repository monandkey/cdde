mod session;
mod store;
mod client;

pub use session::TransactionContext;
pub use store::TransactionStore;
pub use client::DcrClient;

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
        service = "dfl",
        version = env!("CARGO_PKG_VERSION"),
        "Starting Diameter Frontline service"
    );

    // Initialize DCR client
    let dcr_endpoint = std::env::var("DCR_ENDPOINT").unwrap_or_else(|_| "http://[::1]:50051".to_string());
    let _dcr_client = DcrClient::new(dcr_endpoint.clone());
    
    info!("Initialized DCR client pointing to {}", dcr_endpoint);

    // Initialize Session Store
    let _store = TransactionStore::new();

    // TODO: Implement SCTP listener
    // TODO: Implement main event loop
    
    info!("DFL service initialized");
}
