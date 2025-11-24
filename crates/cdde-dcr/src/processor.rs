use crate::routing::RoutingEngine;
use cdde_core::{DiameterPacket, Result};
use cdde_dsl_engine::{Avp, RuleEngine};
use cdde_proto::{ActionType, DiameterPacketAction, DiameterPacketRequest};

/// Packet processor for DCR
pub struct PacketProcessor {
    routing_engine: RoutingEngine,
    rule_engine: Option<RuleEngine>,
}

impl PacketProcessor {
    /// Create new packet processor
    pub fn new(routing_engine: RoutingEngine, rule_engine: Option<RuleEngine>) -> Self {
        Self {
            routing_engine,
            rule_engine,
        }
    }

    /// Process incoming packet request
    pub fn process(&self, request: DiameterPacketRequest) -> Result<DiameterPacketAction> {
        // Parse Diameter packet
        let packet = DiameterPacket::parse(&request.raw_payload)?;

        // Extract routing parameters
        let dest_host = packet
            .find_avp(293)
            .and_then(|avp| String::from_utf8(avp.data.clone()).ok());

        let dest_realm = packet
            .find_avp(283)
            .and_then(|avp| String::from_utf8(avp.data.clone()).ok());

        // Find route
        let route = self.routing_engine.find_route(
            dest_host.as_deref(),
            dest_realm.as_deref(),
            packet.header.application_id,
            packet.header.command_code,
        );

        if route.is_none() {
            // No route found - return error action
            return Ok(DiameterPacketAction {
                action_type: ActionType::Discard,
                target_host_name: None,
                response_payload: vec![],
                original_connection_id: request.connection_id,
            });
        }

        let route = route.unwrap();

        // Apply manipulation rules if configured
        if let Some(ref engine) = self.rule_engine {
            let mut avps: Vec<Avp> = packet
                .avps
                .iter()
                .map(|diameter_avp| Avp {
                    code: diameter_avp.code,
                    value: String::from_utf8_lossy(&diameter_avp.data).to_string(),
                })
                .collect();

            // Process with DSL engine
            engine.process(&mut avps).ok();

            // Update packet AVPs (simplified - in production would need proper conversion)
            // For now, we'll skip the conversion back
        }

        // Serialize modified packet
        let response_payload = packet.serialize();

        Ok(DiameterPacketAction {
            action_type: ActionType::Forward,
            target_host_name: Some(route.target_peer),
            response_payload,
            original_connection_id: request.connection_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::{RouteCondition, RouteEntry};

    #[test]
    fn test_packet_processor() {
        let routes = vec![RouteEntry {
            priority: 10,
            condition: RouteCondition::Default,
            target_pool_id: "default-pool".to_string(),
        }];

        let routing_engine = RoutingEngine::new(routes);
        let processor = PacketProcessor::new(routing_engine, None);

        // Create simple test packet
        let test_packet = vec![
            1, 0, 0, 20, // Header
            0x80, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2,
        ];

        let request = DiameterPacketRequest {
            connection_id: 123,
            vr_id: "vr001".to_string(),
            reception_timestamp: 1234567890,
            raw_payload: test_packet,
            session_tx_id: 456,
        };

        let action = processor.process(request).unwrap();
        assert_eq!(action.action_type, ActionType::Forward);
        assert_eq!(action.target_host_name, Some("default-pool".to_string()));
    }
}
