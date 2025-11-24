mod api;
mod models;
mod repository;

mod db;

pub use db::PostgresRepository;
pub use models::{Dictionary, DictionaryAvp, ManipulationRule, RoutingRule};
pub use repository::{PeerConfig, VirtualRouter};


use tracing::{error, info};

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
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let repository = match PostgresRepository::new(&database_url).await {
        Ok(repo) => repo,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            return;
        }
    };

    // Initialize dictionary manager
    let dictionary_manager = std::sync::Arc::new(cdde_diameter_dict::DictionaryManager::new());

    // Create API router
    let app = api::create_router(repository, dictionary_manager);

    // Start HTTP server
    let addr = "0.0.0.0:3000";
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
