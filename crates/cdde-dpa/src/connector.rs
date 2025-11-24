use cdde_core::{CddeError, Result, Transport};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

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
                    warn!(
                        "Failed to connect to {}: {}. Retrying in {:?}...",
                        self.peer_addr, e, self.reconnect_interval
                    );
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
        info!("Starting handshake with {}", self.peer_addr);
        self.send_cer(socket).await?;
        self.receive_cea(socket).await?;
        info!("Handshake successful with {}", self.peer_addr);

        let mut buffer = [0u8; 4096];

        loop {
            let n = socket.read(&mut buffer).await?;
            if n == 0 {
                return Ok(());
            }

            // Try to parse packet
            match cdde_core::DiameterPacket::parse(&buffer[..n]) {
                Ok(packet) => {
                    // Handle Device-Watchdog-Request (280)
                    if packet.header.command_code == 280 && packet.header.is_request() {
                        info!("Received DWR from {}", self.peer_addr);
                        self.send_dwa(socket, &packet).await?;
                    } else {
                        debug!("Received packet: Command Code {}", packet.header.command_code);
                        // TODO: Forward other requests to DFL/DCR
                    }
                }
                Err(e) => {
                    error!("Failed to parse packet from {}: {}", self.peer_addr, e);
                }
            }
        }
    }

    async fn send_dwa<T: Transport>(&self, socket: &mut T, request: &cdde_core::DiameterPacket) -> Result<()> {
        use cdde_core::{DiameterPacket, DiameterHeader, DiameterAvp};
        use tokio::io::AsyncWriteExt;

        let mut avps = Vec::new();
        // Result-Code (268)
        avps.push(DiameterAvp {
            code: 268,
            flags: 0x40,
            vendor_id: None,
            data: 2001u32.to_be_bytes().to_vec(), // DIAMETER_SUCCESS
        });
        // Origin-Host (264)
        avps.push(DiameterAvp {
            code: 264,
            flags: 0x40,
            vendor_id: None,
            data: b"dpa.example.com".to_vec(),
        });
        // Origin-Realm (296)
        avps.push(DiameterAvp {
            code: 296,
            flags: 0x40,
            vendor_id: None,
            data: b"example.com".to_vec(),
        });

        let header = DiameterHeader {
            version: 1,
            length: 0,
            flags: 0, // Answer
            command_code: 280,
            application_id: 0,
            hop_by_hop_id: request.header.hop_by_hop_id,
            end_to_end_id: request.header.end_to_end_id,
        };

        let packet = DiameterPacket { header, avps };
        let bytes = packet.serialize();
        socket.write_all(&bytes).await?;
        
        info!("Sent DWA to {}", self.peer_addr);
        Ok(())
    }

    async fn send_cer<T: Transport>(&self, socket: &mut T) -> Result<()> {
        use cdde_core::{DiameterPacket, DiameterHeader, DiameterAvp};
        use tokio::io::AsyncWriteExt;

        let mut avps = Vec::new();
        // Origin-Host (264)
        avps.push(DiameterAvp {
            code: 264,
            flags: 0x40, // Mandatory
            vendor_id: None,
            data: b"dpa.example.com".to_vec(),
        });
        // Origin-Realm (296)
        avps.push(DiameterAvp {
            code: 296,
            flags: 0x40,
            vendor_id: None,
            data: b"example.com".to_vec(),
        });
        // Host-IP-Address (257) - simplified (127.0.0.1)
        avps.push(DiameterAvp {
            code: 257,
            flags: 0x40,
            vendor_id: None,
            data: vec![0, 1, 127, 0, 0, 1], 
        });
        // Vendor-Id (266)
        avps.push(DiameterAvp {
            code: 266,
            flags: 0x40,
            vendor_id: None,
            data: 10415u32.to_be_bytes().to_vec(),
        });
        // Product-Name (269)
        avps.push(DiameterAvp {
            code: 269,
            flags: 0,
            vendor_id: None,
            data: b"CDDE-DPA".to_vec(),
        });

        let header = DiameterHeader {
            version: 1,
            length: 0, // Will be calculated
            flags: 0x80, // Request
            command_code: 257,
            application_id: 0,
            hop_by_hop_id: rand::random(),
            end_to_end_id: rand::random(),
        };

        let packet = DiameterPacket { header, avps };
        let bytes = packet.serialize();
        socket.write_all(&bytes).await?;
        
        Ok(())
    }

    async fn receive_cea<T: Transport>(&self, socket: &mut T) -> Result<()> {
        use cdde_core::DiameterPacket;
        
        let mut buffer = [0u8; 4096];
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            return Err(CddeError::ConnectionClosed);
        }
        
        let packet = DiameterPacket::parse(&buffer[..n])?;
        if packet.header.command_code != 257 || packet.header.is_request() {
             return Err(CddeError::InvalidPacket("Expected CEA".to_string()));
        }
        
        // Check Result-Code (268)
        if let Some(avp) = packet.find_avp(268) {
            if avp.data.len() >= 4 {
                let code = u32::from_be_bytes([avp.data[0], avp.data[1], avp.data[2], avp.data[3]]);
                if code != 2001 { // DIAMETER_SUCCESS
                     return Err(CddeError::InvalidPacket(format!("Handshake failed with Result-Code: {}", code)));
                }
            }
        }
        
        Ok(())
    }
}
