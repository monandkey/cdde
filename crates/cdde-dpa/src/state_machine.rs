use serde::{Deserialize, Serialize};

/// Peer connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerState {
    /// Initial state, not connected
    Closed,
    
    /// Attempting to establish connection
    Connecting,
    
    /// CER/CEA exchange in progress
    Negotiating,
    
    /// Connection established and operational
    Open,
    
    /// Connection closing
    Closing,
}

/// Peer state machine
pub struct PeerStateMachine {
    current_state: PeerState,
    peer_id: String,
}

impl PeerStateMachine {
    /// Create new state machine
    pub fn new(peer_id: String) -> Self {
        Self {
            current_state: PeerState::Closed,
            peer_id,
        }
    }

    /// Get current state
    pub fn state(&self) -> PeerState {
        self.current_state
    }

    /// Get peer ID
    pub fn peer_id(&self) -> &str {
        &self.peer_id
    }

    /// Transition to Connecting state
    pub fn connect(&mut self) -> Result<(), String> {
        match self.current_state {
            PeerState::Closed => {
                self.current_state = PeerState::Connecting;
                Ok(())
            }
            _ => Err(format!("Cannot connect from state {:?}", self.current_state)),
        }
    }

    /// Transition to Negotiating state (CER sent)
    pub fn start_negotiation(&mut self) -> Result<(), String> {
        match self.current_state {
            PeerState::Connecting => {
                self.current_state = PeerState::Negotiating;
                Ok(())
            }
            _ => Err(format!("Cannot negotiate from state {:?}", self.current_state)),
        }
    }

    /// Transition to Open state (CEA received successfully)
    pub fn open(&mut self) -> Result<(), String> {
        match self.current_state {
            PeerState::Negotiating => {
                self.current_state = PeerState::Open;
                Ok(())
            }
            _ => Err(format!("Cannot open from state {:?}", self.current_state)),
        }
    }

    /// Transition to Closing state
    pub fn close(&mut self) -> Result<(), String> {
        match self.current_state {
            PeerState::Open | PeerState::Negotiating | PeerState::Connecting => {
                self.current_state = PeerState::Closing;
                Ok(())
            }
            _ => Err(format!("Cannot close from state {:?}", self.current_state)),
        }
    }

    /// Transition to Closed state
    pub fn closed(&mut self) {
        self.current_state = PeerState::Closed;
    }

    /// Check if peer is operational
    pub fn is_operational(&self) -> bool {
        self.current_state == PeerState::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_initialization() {
        let sm = PeerStateMachine::new("peer01".to_string());
        assert_eq!(sm.state(), PeerState::Closed);
        assert_eq!(sm.peer_id(), "peer01");
        assert!(!sm.is_operational());
    }

    #[test]
    fn test_successful_connection_flow() {
        let mut sm = PeerStateMachine::new("peer01".to_string());
        
        // Closed -> Connecting
        assert!(sm.connect().is_ok());
        assert_eq!(sm.state(), PeerState::Connecting);
        
        // Connecting -> Negotiating
        assert!(sm.start_negotiation().is_ok());
        assert_eq!(sm.state(), PeerState::Negotiating);
        
        // Negotiating -> Open
        assert!(sm.open().is_ok());
        assert_eq!(sm.state(), PeerState::Open);
        assert!(sm.is_operational());
    }

    #[test]
    fn test_invalid_state_transitions() {
        let mut sm = PeerStateMachine::new("peer01".to_string());
        
        // Cannot negotiate from Closed
        assert!(sm.start_negotiation().is_err());
        
        // Cannot open from Closed
        assert!(sm.open().is_err());
    }

    #[test]
    fn test_close_from_open() {
        let mut sm = PeerStateMachine::new("peer01".to_string());
        
        sm.connect().unwrap();
        sm.start_negotiation().unwrap();
        sm.open().unwrap();
        
        // Open -> Closing
        assert!(sm.close().is_ok());
        assert_eq!(sm.state(), PeerState::Closing);
        
        // Closing -> Closed
        sm.closed();
        assert_eq!(sm.state(), PeerState::Closed);
    }
}
