use cdde_proto::{ActionType, DiameterPacketAction, DiameterPacketRequest};
use std::time::Duration;

#[tokio::test]
async fn test_e2e_flow() {
    // 1. Start Mock DCR (gRPC Server)
    let dcr_addr = "[::1]:50052"; // Use different port than default
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let dcr_server = async move {
        use cdde_proto::core_router_service_server::{CoreRouterService, CoreRouterServiceServer};
        use tonic::{Request, Response, Status};

        struct MockDcr {
            tx: tokio::sync::mpsc::Sender<()>,
        }

        #[tonic::async_trait]
        impl CoreRouterService for MockDcr {
            async fn process_packet(
                &self,
                request: Request<DiameterPacketRequest>,
            ) -> Result<Response<DiameterPacketAction>, Status> {
                // Signal that we received a packet
                self.tx.send(()).await.ok();

                Ok(Response::new(DiameterPacketAction {
                    action_type: ActionType::Reply as i32,
                    target_host_name: "".to_string(),
                    response_payload: request.into_inner().raw_payload, // Echo
                    original_connection_id: 0,
                }))
            }
        }

        let service = MockDcr { tx };
        tonic::transport::Server::builder()
            .add_service(CoreRouterServiceServer::new(service))
            .serve(dcr_addr.parse().unwrap())
            .await
            .unwrap();
    };

    tokio::spawn(dcr_server);

    // Wait for DCR to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 2. Verify DCR connection
    let mut client = cdde_proto::core_router_service_client::CoreRouterServiceClient::connect(
        format!("http://{dcr_addr}"),
    )
    .await
    .expect("Failed to connect to Mock DCR");

    let request = DiameterPacketRequest {
        connection_id: 1,
        vr_id: "test".to_string(),
        reception_timestamp: 0,
        raw_payload: vec![1, 2, 3, 4], // Dummy payload
        session_tx_id: 0,
    };

    let response = client
        .process_packet(request)
        .await
        .expect("gRPC call failed");
    let action = response.into_inner();

    assert_eq!(action.action_type, ActionType::Reply as i32);
    assert_eq!(action.response_payload, vec![1, 2, 3, 4]);

    // Verify DCR received it
    assert!(rx.recv().await.is_some());
}
