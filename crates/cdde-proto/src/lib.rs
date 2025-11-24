// Internal protocol definitions for CDDE
// Note: This is a temporary implementation using serde.
// TODO: Migrate to Protocol Buffers once protoc is available.

use serde::{Deserialize, Serialize};

// ========================================
// DFL to DCR Messages
// ========================================

/// Request message from DFL to DCR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiameterPacketRequest {
    /// Unique ID for SCTP connection within DFL
    pub connection_id: u64,

    /// Virtual Router ID determined by DFL from received IP
    pub vr_id: String,

    /// Reception timestamp at DFL (nanoseconds)
    pub reception_timestamp: u64,

    /// Raw Diameter packet (binary data)
    pub raw_payload: Vec<u8>,

    /// Session tracking ID assigned by DFL
    pub session_tx_id: u64,
}

/// Action message from DCR to DFL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiameterPacketAction {
    /// Action to perform
    pub action_type: ActionType,

    /// Target host name for FORWARD action
    pub target_host_name: Option<String>,

    /// Final Diameter packet to send (after manipulation)
    pub response_payload: Vec<u8>,

    /// Original connection ID for REPLY action
    pub original_connection_id: u64,
}

/// Action type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Forward, // Forward to next hop
    Reply,   // Send immediate response
    Discard, // Discard packet
}

// ========================================
// DPA to DFL Messages
// ========================================

/// Peer status update request from DPA to DFL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStatusRequest {
    /// Peer node identifier
    pub peer_node_id: String,

    /// Current peer status
    pub current_status: PeerStatus,

    /// Affected Virtual Router IDs
    pub virtual_router_ids: Vec<String>,
}

/// Peer status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Up,
    Down,
}

/// Response to peer status update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResponse {
    pub success: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diameter_packet_request_serialization() {
        let req = DiameterPacketRequest {
            connection_id: 123,
            vr_id: "vr001".to_string(),
            reception_timestamp: 1234567890,
            raw_payload: vec![1, 2, 3, 4],
            session_tx_id: 456,
        };

        let json = serde_json::to_string(&req).unwrap();
        let deserialized: DiameterPacketRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req.connection_id, deserialized.connection_id);
        assert_eq!(req.vr_id, deserialized.vr_id);
    }

    #[test]
    fn test_action_type() {
        assert_eq!(ActionType::Forward, ActionType::Forward);
        assert_ne!(ActionType::Forward, ActionType::Reply);
    }

    #[test]
    fn test_peer_status() {
        let req = PeerStatusRequest {
            peer_node_id: "peer01".to_string(),
            current_status: PeerStatus::Up,
            virtual_router_ids: vec!["vr001".to_string()],
        };

        assert_eq!(req.current_status, PeerStatus::Up);
    }
}
