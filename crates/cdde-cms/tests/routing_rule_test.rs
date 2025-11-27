use cdde_cms::models::{PeerConfig, RoutingRule, VirtualRouter};
use reqwest::StatusCode;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_routing_rule_lifecycle() {
    let client = common::get_client();
    let base_url = common::get_base_url();

    // Setup: VR and Peer
    let vr_id = uuid::Uuid::new_v4().to_string();
    let vr = VirtualRouter {
        id: vr_id.clone(),
        hostname: format!("vr-rr-{}.example.com", vr_id),
        realm: "example.com".to_string(),
        timeout_ms: 3000,
    };
    client.post(format!("{}/vrs", base_url)).json(&vr).send().await.unwrap();

    let peer_id = uuid::Uuid::new_v4().to_string();
    let peer = PeerConfig {
        id: peer_id.clone(),
        hostname: format!("peer-rr-{}.example.com", peer_id),
        realm: "example.com".to_string(),
        ip_address: "127.0.0.1".to_string(),
        port: 3868,
        vr_id: Some(vr_id.clone()),
    };
    client.post(format!("{}/peers", base_url)).json(&peer).send().await.unwrap();

    // 1. Create Routing Rule
    // Note: We send peer_id, but model expects it mapped to target_pool (handled by serde rename)
    // We also omit vr_id in payload to test auto-population from path
    let rule_payload = json!({
        "priority": 10,
        "destination_realm": "dest.example.com",
        "peer_id": peer_id,
        "target_pool": "should_be_ignored_if_peer_id_is_used" // Actually peer_id maps to target_pool field in struct
    });
    // Wait, if I send both, serde might error or pick one.
    // The struct has `peer_id` field with `rename="peer_id"`. So `target_pool` key in JSON will be ignored (unless I have another field).
    // I should send `peer_id`.

    let rule_payload = json!({
        "priority": 10,
        "destination_realm": "dest.example.com",
        "peer_id": peer_id
    });

    let res = client
        .post(format!("{}/vrs/{}/routing-rules", base_url, vr_id))
        .json(&rule_payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::CREATED);
    let body: serde_json::Value = res.json().await.unwrap();
    let rule_id = body["id"].as_i64().expect("ID not found");

    // 2. Get Routing Rule
    let res = client
        .get(format!("{}/routing-rules/{}", base_url, rule_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);
    let fetched_rule: RoutingRule = res.json().await.expect("Failed to parse JSON");
    assert_eq!(fetched_rule.vr_id, vr_id);
    assert_eq!(fetched_rule.peer_id, peer_id);

    // 3. List Routing Rules
    let res = client
        .get(format!("{}/vrs/{}/routing-rules", base_url, vr_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);
    let rules: Vec<RoutingRule> = res.json().await.unwrap();
    assert!(rules.iter().any(|r| r.id as i64 == rule_id));

    // 4. Update Routing Rule
    let mut updated_payload = json!({
        "vr_id": vr_id,
        "priority": 20,
        "destination_realm": "dest.example.com",
        "peer_id": peer_id
    });

    let res = client
        .put(format!("{}/routing-rules/{}", base_url, rule_id))
        .json(&updated_payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);

    // 5. Delete Routing Rule
    let res = client
        .delete(format!("{}/routing-rules/{}", base_url, rule_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Cleanup
    client.delete(format!("{}/vrs/{}", base_url, vr_id)).send().await.unwrap();
    client.delete(format!("{}/peers/{}", base_url, peer_id)).send().await.unwrap();
}
