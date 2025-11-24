#[cfg(test)]
mod integration_tests {
    use crate::network::TcpServer;
    use crate::store::TransactionStore;
    use cdde_core::{DiameterHeader, DiameterPacket};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;

    #[tokio::test]
    async fn test_tcp_connection_and_packet_exchange() {
        // 1. Start DFL Server
        let addr = "127.0.0.1:3869"; // Use different port for test
        let store = Arc::new(TransactionStore::new());
        let server = TcpServer::new(addr.to_string(), store);

        let server_handle = tokio::spawn(async move {
            server.start().await.unwrap();
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 2. Connect with TCP Client (simulating DPA)
        let mut stream = TcpStream::connect(addr).await.unwrap();

        // 3. Send a Diameter Packet
        let packet = DiameterPacket {
            header: DiameterHeader {
                version: 1,
                length: 20,
                flags: 0x80,       // Request
                command_code: 280, // DWR
                application_id: 0,
                hop_by_hop_id: 123,
                end_to_end_id: 456,
            },
            avps: vec![],
        };
        let data = packet.serialize();
        stream.write_all(&data).await.unwrap();

        // 4. Wait a bit to ensure server processed it (check logs manually or add feedback mechanism later)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Cleanup
        server_handle.abort();
    }
}
