mod repository;
mod api;

pub use repository::{ConfigRepository, VirtualRouter, PeerConfig};

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
        service = "cms",
        version = env!("CARGO_PKG_VERSION"),
        "Starting Config & Management Service"
    );

    // Initialize repository
    let repository = ConfigRepository::new();
    
    // Create API router
    let app = api::create_router(repository);

    // Start HTTP server
    let addr = "0.0.0.0:3000";
    info!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
