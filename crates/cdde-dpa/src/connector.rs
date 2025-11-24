use cdde_core::{Result, Transport};
use tokio::net::TcpStream;
use tokio::io::AsyncReadExt;
use tracing::{info, error, warn};
use std::time::Duration;

/// TCP Client for Diameter peer connections
pub struct TcpClient {
    peer_addr: String,
    reconnect_interval: Duration,
}

impl TcpClient {
    /// Create new TCP client
    pub fn new(peer_addr: String) -> Self {
        Self {
            peer_addr,
            reconnect_interval: Duration::from_secs(5),
        }
    }

    /// Start connection loop
    pub async fn start(&self) {
        info!("Starting DPA connector to {}", self.peer_addr);

        loop {
            match self.connect().await {
                Ok(mut socket) => {
                    info!("Connected to {}", self.peer_addr);
                    if let Err(e) = self.handle_connection(&mut socket).await {
                        error!("Connection lost: {}", e);
                    }
                }
                Err(e) => {
                    warn!("Failed to connect to {}: {}. Retrying in {:?}...", 
                        self.peer_addr, e, self.reconnect_interval);
                }
            }
            
            tokio::time::sleep(self.reconnect_interval).await;
        }
    }

    /// Establish connection
    async fn connect(&self) -> Result<TcpStream> {
        let stream = TcpStream::connect(&self.peer_addr).await?;
        Ok(stream)
    }

    /// Handle connected session
    async fn handle_connection<T: Transport>(&self, socket: &mut T) -> Result<()> {
        // TODO: Implement handshake (CER/CEA)
        
        let mut buffer = [0u8; 4096];
        
        loop {
            let n = socket.read(&mut buffer).await?;
            if n == 0 {
                return Ok(());
            }
            
            // Echo back for now (mock behavior) or process
            // In real DPA, we would handle DWR/DWA and forward requests
        }
    }
}
