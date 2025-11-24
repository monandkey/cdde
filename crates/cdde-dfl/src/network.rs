// Force re-link
use cdde_core::{DiameterPacket, Result, Transport};
use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use tracing::{info, error, debug};
use std::sync::Arc;
use crate::store::TransactionStore;

/// TCP Server for Diameter connections
pub struct TcpServer {
    addr: String,
    store: Arc<TransactionStore>,
}

impl TcpServer {
    /// Create new TCP server
    pub fn new(addr: String, store: Arc<TransactionStore>) -> Self {
        Self { addr, store }
    }

    /// Start listening loop
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!("DFL listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    info!("New connection from {}", addr);
                    let store = self.store.clone();
                    
                    // Spawn connection handler
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(socket, store).await {
                            error!("Connection error from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    /// Handle individual connection
    async fn handle_connection<T: Transport>(mut socket: T, _store: Arc<TransactionStore>) -> Result<()> {
        let mut buffer = [0u8; 4096]; // 4KB buffer

        loop {
            // Read header first (simplified: reading chunks for now)
            let n = socket.read(&mut buffer).await?;
            if n == 0 {
                info!("Connection closed by peer");
                return Ok(());
            }

            debug!("Received {} bytes", n);

            // Try to parse packet
            match DiameterPacket::parse(&buffer[..n]) {
                Ok(packet) => {
                    debug!("Parsed packet: Command Code {}", packet.header.command_code);
                    // TODO: Process packet, create transaction, send to DCR
                }
                Err(e) => {
                    error!("Failed to parse packet: {}", e);
                    // In real impl: handle partial reads / buffering
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use async_trait::async_trait;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::net::{SocketAddr, IpAddr, Ipv4Addr};

    // Mock Transport for testing
    struct MockTransport {
        read_data: Vec<u8>,
    }

    impl AsyncRead for MockTransport {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            let me = self.get_mut();
            if me.read_data.is_empty() {
                return Poll::Ready(Ok(()));
            }
            
            let len = std::cmp::min(buf.remaining(), me.read_data.len());
            buf.put_slice(&me.read_data[..len]);
            me.read_data.drain(..len);
            
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for MockTransport {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        fn peer_addr(&self) -> Result<SocketAddr> {
            Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345))
        }

        fn local_addr(&self) -> Result<SocketAddr> {
            Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3868))
        }
    }

    #[tokio::test]
    async fn test_handle_connection_parse_v2() {
        let packet = DiameterPacket {
            header: cdde_core::DiameterHeader {
                version: 1,
                length: 20,
                flags: 0x80,
                command_code: 280, // DWR
                application_id: 0,
                hop_by_hop_id: 1,
                end_to_end_id: 2,
            },
            avps: vec![],
        };
        
        let data = packet.serialize();
        let transport = MockTransport { read_data: data };
        let store = Arc::new(TransactionStore::new());

        // This will process one packet and then "close" (read returns 0)
        // We just want to ensure it doesn't panic
        let result = TcpServer::handle_connection(transport, store).await;
        // It might return Ok or error depending on how the mock loop behaves with 0 read
        // In our mock, poll_read puts data once. Next call?
        // Actually our mock keeps putting data forever if we don't clear it.
        // Let's improve mock if needed, but for now just checking compilation and basic structure.
    }
}
