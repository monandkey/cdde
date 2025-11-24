mod processor;
mod routing;

pub use processor::PacketProcessor;
pub use routing::{RouteCondition, RouteEntry, RoutingDecision, RoutingEngine};


use cdde_proto::{DiameterPacketAction, DiameterPacketRequest};
use cdde_proto::core_router_service_server::CoreRouterService;
use std::sync::Arc;
use tracing::info;
use tonic::{Request, Response, Status};

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
}

#[tonic::async_trait]
impl CoreRouterService for CoreRouterServiceImpl {
    async fn process_packet(
        &self,
        request: Request<DiameterPacketRequest>,
    ) -> Result<Response<DiameterPacketAction>, Status> {
        let req = request.into_inner();
        let action = self.processor
            .process(req)
            .map_err(|e| Status::internal(format!("Processing error: {e}")))?;
        Ok(Response::new(action))
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
    let processor = PacketProcessor::new(routing_engine, None);

    info!("DCR service initialized with packet processor");

    // Start gRPC server
    let addr = "[::1]:50051".parse().unwrap();
    let service = CoreRouterServiceImpl::new(processor);
    
    info!("Starting gRPC server on {}", addr);
    
    tonic::transport::Server::builder()
        .add_service(cdde_proto::core_router_service_server::CoreRouterServiceServer::new(service))
        .serve(addr)
        .await
        .unwrap();
}
