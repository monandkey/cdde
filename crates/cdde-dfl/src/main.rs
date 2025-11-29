// Modules are defined in lib.rs

use cdde_dfl::app::client::DcrClient;
use cdde_dfl::app::network::TcpServer;
use cdde_dfl::app::session::TransactionContext;
use cdde_dfl::app::store::TransactionStore;

use std::sync::Arc;
use tracing::info;
use cdde_dfl::core::types::SessionConfig;
use cdde_dfl::runtime::session_actor::SessionActor;

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
    let dcr_endpoint =
        std::env::var("DCR_ENDPOINT").unwrap_or_else(|_| "http://[::1]:50051".to_string());
    let _dcr_client = DcrClient::new(dcr_endpoint.clone());

    info!("Initialized DCR client pointing to {}", dcr_endpoint);

    // Initialize Session Store
    let store = Arc::new(TransactionStore::new());

    // Initialize Session Actor
    let (actor_tx, actor_rx) = tokio::sync::mpsc::channel(100);
    let (outbound_tx, mut outbound_rx) = tokio::sync::mpsc::channel(100);
    
    let session_config = SessionConfig {
        timeout_duration: std::time::Duration::from_secs(30),
    };
    
    let actor = SessionActor::new(session_config, actor_rx, outbound_tx);
    tokio::spawn(actor.run());
    
    info!("Session Actor started");

    // Spawn a task to handle outbound actions from SessionActor
    // TODO: この実装では outbound_rx からアクションを受け取り、実際の処理を行う
    // - ForwardToDcr: DCR Client でメッセージ送信
    // - ReplyWith3002Error: TCP Socket で 3002 エラー応答を送信
    // - RemoveSession: セッションストアからエントリ削除
    tokio::spawn(async move {
        while let Some(_action) = outbound_rx.recv().await {
            // TODO: Handle SessionAction here
            // match action { ... }
        }
    });

    // Start TCP Server
    // TODO: TcpServer に actor_tx を渡して、受信したパケットを SessionActor に送信できるようにする
    // 現在は actor_tx が未使用だが、本来は TcpServer::new(bind_addr, store, actor_tx) のように渡すべき
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3868".to_string());
    let server = TcpServer::new(bind_addr.clone(), store);

    info!("Starting TCP listener on {}", bind_addr);

    if let Err(e) = server.start().await {
        info!("Server error: {}", e);
    }
}
