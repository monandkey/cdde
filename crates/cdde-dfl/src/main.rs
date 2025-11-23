mod session;
mod store;

pub use session::TransactionContext;
pub use store::TransactionStore;

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

    // TODO: Implement SCTP listener
    // TODO: Implement gRPC client to DCR
    // TODO: Implement main event loop
    
    info!("DFL service initialized");
}
