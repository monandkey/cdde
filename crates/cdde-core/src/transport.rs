use crate::error::Result;
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};

/// Abstract transport layer trait
/// Allows switching between TCP and SCTP (or mocks) transparently
#[async_trait]
pub trait Transport: AsyncRead + AsyncWrite + Send + Unpin {
    /// Get remote peer address
    fn peer_addr(&self) -> Result<SocketAddr>;

    /// Get local address
    fn local_addr(&self) -> Result<SocketAddr>;
}

// Implement Transport for tokio::net::TcpStream
#[async_trait]
impl Transport for tokio::net::TcpStream {
    fn peer_addr(&self) -> Result<SocketAddr> {
        Ok(self.peer_addr()?)
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr()?)
    }
}
