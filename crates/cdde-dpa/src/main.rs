mod state_machine;

pub use state_machine::{PeerStateMachine, PeerState};

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

    // TODO: Load peer configuration
    // TODO: Implement alive monitoring (DWR/DWA)
    // TODO: Implement SCTP heartbeat monitoring
    // TODO: Implement peer status notification to DFL
    
    info!("DPA service initialized");
}
