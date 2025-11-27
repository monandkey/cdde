use cdde_cms::models::{PeerConfig, VirtualRouter};
use reqwest::StatusCode;

mod common;

#[tokio::test]
async fn test_peer_lifecycle() {
    let client = common::get_client();
    let base_url = common::get_base_url();

    // Setup: Create a VR for the peer
    let vr_id = uuid::Uuid::new_v4().to_string();
    let vr = VirtualRouter {
        id: vr_id.clone(),
        hostname: format!("vr-for-peer-{}.example.com", vr_id),
        realm: "example.com".to_string(),
        timeout_ms: 3000,
    };
    client
        .post(format!("{}/vrs", base_url))
        .json(&vr)
        .send()
        .await
        .unwrap();

    // 1. Create Peer
    let peer_hostname = format!("peer-{}.example.com", uuid::Uuid::new_v4());
    let peer = PeerConfig {
        id: "".to_string(), // Auto-generate
        hostname: peer_hostname.clone(),
        realm: "example.com".to_string(),
        ip_address: "127.0.0.1".to_string(),
        port: 3868,
        vr_id: Some(vr_id.clone()),
    };

    let res = client
        .post(format!("{}/peers", base_url))
        .json(&peer)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::CREATED);

    // 2. List Peers and find the created one
    let res = client
        .get(format!("{}/peers", base_url))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);
    let peers: Vec<PeerConfig> = res.json().await.expect("Failed to parse JSON");
    let created_peer = peers
        .iter()
        .find(|p| p.hostname == peer_hostname)
        .expect("Peer not found");
    let peer_id = created_peer.id.clone();
    assert!(!peer_id.is_empty());
    assert_eq!(created_peer.vr_id, Some(vr_id.clone()));

    // 3. Get Peer by ID
    let res = client
        .get(format!("{}/peers/{}", base_url, peer_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);
    let fetched_peer: PeerConfig = res.json().await.expect("Failed to parse JSON");
    assert_eq!(fetched_peer.id, peer_id);

    // 4. Update Peer
    let mut updated_peer = fetched_peer.clone();
    updated_peer.port = 3869;

    let res = client
        .put(format!("{}/peers/{}", base_url, peer_id))
        .json(&updated_peer)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);

    // Verify update
    let res = client
        .get(format!("{}/peers/{}", base_url, peer_id))
        .send()
        .await
        .expect("Failed to send request");

    let fetched_updated_peer: PeerConfig = res.json().await.expect("Failed to parse JSON");
    assert_eq!(fetched_updated_peer.port, 3869);

    // 5. Create another peer with SAME hostname but different VR (or same VR)
    let peer2 = PeerConfig {
        id: "".to_string(),
        hostname: peer_hostname.clone(), // Same hostname
        realm: "example.com".to_string(),
        ip_address: "127.0.0.2".to_string(),
        port: 3868,
        vr_id: Some(vr_id.clone()),
    };

    let res = client
        .post(format!("{}/peers", base_url))
        .json(&peer2)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::CREATED);

    // Verify both exist
    let res = client
        .get(format!("{}/peers", base_url))
        .send()
        .await
        .expect("Failed to send request");
    let peers: Vec<PeerConfig> = res.json().await.unwrap();
    let matching_peers: Vec<_> = peers
        .iter()
        .filter(|p| p.hostname == peer_hostname)
        .collect();
    assert_eq!(matching_peers.len(), 2);

    // 6. Delete Peer
    let res = client
        .delete(format!("{}/peers/{}", base_url, peer_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Cleanup VR
    client
        .delete(format!("{}/vrs/{}", base_url, vr_id))
        .send()
        .await
        .unwrap();
}
