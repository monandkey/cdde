use cdde_dcr_core::router::{RouterCore, RouteAction};
use cdde_shared::DiameterMessage;
use cdde_core::{DiameterPacket, DiameterAvp};
use cdde_proto::{
    core_router_service_server::{CoreRouterService, CoreRouterServiceServer},
    DiameterPacketAction, DiameterPacketRequest, ActionType,
};
use arc_swap::ArcSwap;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct DcrService {
    // ★ Lock-Free Configuration Update
    core: Arc<ArcSwap<RouterCore>>,
}

impl DcrService {
    pub fn new(initial_core: RouterCore) -> Self {
        Self {
            core: Arc::new(ArcSwap::from_pointee(initial_core)),
        }
    }

    // 設定更新API (管理プレーンから呼ばれる)
    pub fn update_config(&self, new_core: RouterCore) {
        self.core.store(Arc::new(new_core));
    }

    // gRPCサーバーを作成
    pub fn into_grpc_server(self) -> CoreRouterServiceServer<Self> {
        CoreRouterServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl CoreRouterService for DcrService {
    async fn process_packet(
        &self,
        request: Request<DiameterPacketRequest>,
    ) -> Result<Response<DiameterPacketAction>, Status> {
        let req = request.into_inner();

        // Parse Diameter packet
        let packet = DiameterPacket::parse(&req.raw_payload).map_err(|e| {
            Status::invalid_argument(format!("Failed to parse Diameter packet: {}", e))
        })?;

        // Convert to DiameterMessage
        let msg = DiameterMessage {
            version: packet.header.version,
            flags: packet.header.flags,
           command_code: packet.header.command_code,
            application_id: packet.header.application_id,
            hop_by_hop_id: packet.header.hop_by_hop_id,
            end_to_end_id: packet.header.end_to_end_id,
            is_request: packet.header.is_request(),
            avps: packet
                .avps
                .iter()
                .map(|avp| cdde_shared::Avp {
                    code: avp.code,
                    flags: avp.flags,
                    length: (avp.data.len() + 8) as u32,
                    vendor_id: avp.vendor_id,
                    data: bytes::Bytes::from(avp.data.clone()),
                })
                .collect(),
        };

        // Process with Core
        let current_core = self.core.load();
        let (processed_msg, action) = current_core.process(msg);

        // Convert back to bytes
        let response_payload = DiameterPacket {
            header: cdde_core::DiameterHeader {
                version: processed_msg.version,
                length: 0, // Will be recalculated
                flags: processed_msg.flags,
                command_code: processed_msg.command_code,
                application_id: processed_msg.application_id,
                hop_by_hop_id: processed_msg.hop_by_hop_id,
                end_to_end_id: processed_msg.end_to_end_id,
            },
            avps: processed_msg
                .avps
                .iter()
                .map(|avp| DiameterAvp {
                    code: avp.code,
                    flags: avp.flags,
                    vendor_id: avp.vendor_id,
                    data: avp.data.to_vec(),
                })
                .collect(),
        }
        .serialize();

        // Build action
        let grpc_action = match action {
            RouteAction::Forward(peer) => DiameterPacketAction {
                action_type: ActionType::Forward as i32,
                target_host_name: peer,
                response_payload,
                original_connection_id: req.connection_id,
            },
            RouteAction::Discard => DiameterPacketAction {
                action_type: ActionType::Discard as i32,
                target_host_name: String::new(),
                response_payload: vec![],
                original_connection_id: req.connection_id,
            },
            RouteAction::ReplyError(_code) => DiameterPacketAction {
                action_type: ActionType::Reply as i32,
                target_host_name: String::new(),
                response_payload,
                original_connection_id: req.connection_id,
            },
        };

        Ok(Response::new(grpc_action))
    }
}
