mod processor;
mod routing;

pub use processor::PacketProcessor;
pub use routing::{RouteCondition, RouteEntry, RoutingDecision, RoutingEngine};


use cdde_proto::{DiameterPacketAction, DiameterPacketRequest};
use std::sync::Arc;
use tracing::info;

/// Simple in-memory gRPC service implementation
pub struct CoreRouterServiceImpl {
    processor: Arc<PacketProcessor>,
}

impl CoreRouterServiceImpl {
    pub fn new(processor: PacketProcessor) -> Self {
        Self {
            processor: Arc::new(processor),
        }
    }

    pub async fn process_packet(
        &self,
        request: DiameterPacketRequest,
    ) -> Result<DiameterPacketAction, tonic::Status> {
        self.processor
            .process(request)
            .map_err(|e| tonic::Status::internal(format!("Processing error: {e}")))
    }
}

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

    // Create default routing configuration
    let routes = vec![RouteEntry {
        priority: 100,
        condition: RouteCondition::Default,
        target_pool_id: "default-pool".to_string(),
    }];

    let routing_engine = RoutingEngine::new(routes);
    let _processor = PacketProcessor::new(routing_engine, None);

    info!("DCR service initialized with packet processor");

    // TODO: Start gRPC server
    // let addr = "[::1]:50051".parse().unwrap();
    // let service = CoreRouterServiceImpl::new(processor);
    // Server::builder()
    //     .add_service(CoreRouterServiceServer::new(service))
    //     .serve(addr)
    //     .await
    //     .unwrap();
}
