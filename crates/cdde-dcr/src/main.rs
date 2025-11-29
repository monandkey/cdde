use cdde_dcr_core::router::{RouterCore, RouteEntry};
use cdde_dcr_core::manipulation::ManipulationEngine;
use cdde_dcr_runtime::service::DcrService;
use tracing::info;

#[tokio::main]
async fn main() {
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
    
    info!("DCR Service initialized. Listening on 0.0.0.0:50051 (Mock)");
    
    // gRPCサーバー起動 (モック)
    // tonic::transport::Server::builder()
    //    .add_service(DcrServer::new(service))
    //    .serve(addr)
    //    .await?;
    
    // 簡易的に待機
    tokio::signal::ctrl_c().await.unwrap();
    info!("Shutting down DCR service");
}
