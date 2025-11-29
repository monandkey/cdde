use cdde_dcr::core::router::{RouterCore, RouteEntry};
use cdde_dcr::core::manipulation::ManipulationEngine;
use cdde_dcr::runtime::service::DcrService;
use cdde_proto::cdde::core_router_service_server::CoreRouterServiceServer;
use tonic::transport::Server;
use tracing::info;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    cdde_logging::init();

    info!(
        service = "dcr",
        version = env!("CARGO_PKG_VERSION"),
        "Starting Diameter Core Router service"
    );

    // 初期設定のロード (本来はファイルやDBから)
    let routes = vec![
        RouteEntry {
            dest_realm: "example.com".to_string(),
            target_peer: "peer-a".to_string(),
        }
    ];
    let manipulator = ManipulationEngine::new(vec![]);
    let core = RouterCore::new(routes, manipulator);

    // Service起動
    let service = DcrService::new(core);
    let grpc_server = service.into_grpc_server();
    
    let addr = std::env::var("DCR_BIND_ADDR")
        .unwrap_or_else(|_| "[::1]:50051".to_string())
        .parse()?;

    info!("DCR gRPC Service listening on {}", addr);
    
    Server::builder()
        .add_service(grpc_server)
        .serve(addr)
        .await?;

    Ok(())
}
