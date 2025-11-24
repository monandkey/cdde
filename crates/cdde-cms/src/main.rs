mod api;
mod models;

mod db;
mod error;

pub use db::PostgresRepository;
pub use error::AppError;
pub use models::{
    Dictionary, DictionaryAvp, ManipulationRule, PeerConfig, RoutingRule, VirtualRouter,
};

use tracing::{error, info};

use axum::Router;

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
    let api_router = api::create_router(repository, dictionary_manager);

    // Swagger UI
    use api::ApiDoc;
    use utoipa::OpenApi;
    use utoipa_swagger_ui::SwaggerUi;

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(api_router);

    // Start HTTP server
    let addr = "0.0.0.0:3000";
    info!("Starting CMS server on 0.0.0.0:3000");
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
