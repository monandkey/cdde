mod routing;

pub use routing::{RoutingEngine, RoutingDecision, RouteEntry, RouteCondition};

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
        service = "dcr",
        version = env!("CARGO_PKG_VERSION"),
        "Starting Diameter Core Router service"
    );

    // TODO: Load routing configuration
    // TODO: Implement gRPC server
    // TODO: Implement packet processing pipeline
    
    info!("DCR service initialized");
}
