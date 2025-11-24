// Integration tests for cdde-cms
// Note: These tests require a PostgreSQL database to be running
// Set DATABASE_URL environment variable to run these tests
// Example: DATABASE_URL=postgres://postgres:postgres@localhost/cdde_test cargo test

use cdde_cms::{PostgresRepository, VirtualRouter, PeerConfig};

// Helper function to get test database URL
fn get_test_db_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/cdde_test".to_string())
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_vr_crud_operations() {
    let db_url = get_test_db_url();
    
    // Create repository
    let repo = PostgresRepository::new(&db_url)
        .await
        .expect("Failed to create repository. Make sure TEST_DATABASE_URL is set and database is running.");

    // Test CREATE
    let vr = VirtualRouter {
        id: "test_vr1".to_string(),
        hostname: "test-host.example.com".to_string(),
        realm: "example.com".to_string(),
        timeout_ms: 3000,
    };
    
    assert!(repo.add_vr(vr.clone()).await, "Failed to create VR");

    // Test READ
    let fetched_vr = repo.get_vr("test_vr1").await;
    assert!(fetched_vr.is_some(), "Failed to fetch VR");
    let fetched_vr = fetched_vr.unwrap();
    assert_eq!(fetched_vr.id, "test_vr1");
    assert_eq!(fetched_vr.hostname, "test-host.example.com");
    assert_eq!(fetched_vr.realm, "example.com");
    assert_eq!(fetched_vr.timeout_ms, 3000);

    // Test UPDATE
    let updated_vr = VirtualRouter {
        id: "test_vr1".to_string(),
        hostname: "updated-host.example.com".to_string(),
        realm: "updated.example.com".to_string(),
        timeout_ms: 5000,
    };
    
    assert!(repo.update_vr(updated_vr).await, "Failed to update VR");
    
    let fetched_vr = repo.get_vr("test_vr1").await.unwrap();
    assert_eq!(fetched_vr.hostname, "updated-host.example.com");
    assert_eq!(fetched_vr.realm, "updated.example.com");
    assert_eq!(fetched_vr.timeout_ms, 5000);

    // Test LIST
    let vrs = repo.get_all_vrs().await;
    assert!(!vrs.is_empty(), "VR list should not be empty");

    // Test DELETE
    assert!(repo.delete_vr("test_vr1").await, "Failed to delete VR");
    assert!(repo.get_vr("test_vr1").await.is_none(), "VR should be deleted");
}

#[tokio::test]
#[ignore]
async fn test_peer_crud_operations() {
    let db_url = get_test_db_url();
    let repo = PostgresRepository::new(&db_url)
        .await
        .expect("Failed to create repository");

    // Test CREATE
    let peer = PeerConfig {
        hostname: "peer.example.com".to_string(),
        realm: "example.com".to_string(),
        ip_address: "192.168.1.10".to_string(),
        port: 3868,
    };
    
    assert!(repo.add_peer(peer.clone()).await, "Failed to create peer");

    // Test READ
    let fetched_peer = repo.get_peer("peer.example.com").await;
    assert!(fetched_peer.is_some(), "Failed to fetch peer");
    let fetched_peer = fetched_peer.unwrap();
    assert_eq!(fetched_peer.hostname, "peer.example.com");
    assert_eq!(fetched_peer.ip_address, "192.168.1.10");
    assert_eq!(fetched_peer.port, 3868);

    // Test LIST
    let peers = repo.get_all_peers().await;
    assert!(!peers.is_empty(), "Peer list should not be empty");

    // Test DELETE
    assert!(repo.delete_peer("peer.example.com").await, "Failed to delete peer");
    assert!(repo.get_peer("peer.example.com").await.is_none(), "Peer should be deleted");
}

#[tokio::test]
#[ignore]
async fn test_dictionary_operations() {
    let db_url = get_test_db_url();
    let repo = PostgresRepository::new(&db_url)
        .await
        .expect("Failed to create repository");

    // Test CREATE
    let xml_content = r#"
    <dictionary>
        <avp name="Test-AVP" code="10001" type="Unsigned32" vendor-id="9999"/>
    </dictionary>
    "#.to_string();
    
    let dict_id = repo.save_dictionary(
        "test-dict".to_string(),
        "1.0".to_string(),
        xml_content.clone()
    ).await;
    
    assert!(dict_id.is_some(), "Failed to create dictionary");
    let dict_id = dict_id.unwrap();

    // Test READ
    let fetched_dict = repo.get_dictionary(dict_id).await;
    assert!(fetched_dict.is_some(), "Failed to fetch dictionary");
    let fetched_dict = fetched_dict.unwrap();
    assert_eq!(fetched_dict.name, "test-dict");
    assert_eq!(fetched_dict.version, "1.0");

    // Test LIST
    let dicts = repo.list_dictionaries().await;
    assert!(!dicts.is_empty(), "Dictionary list should not be empty");

    // Test DELETE
    assert!(repo.delete_dictionary(dict_id).await, "Failed to delete dictionary");
    assert!(repo.get_dictionary(dict_id).await.is_none(), "Dictionary should be deleted");
}

#[tokio::test]
#[ignore]
async fn test_routing_rule_operations() {
    let db_url = get_test_db_url();
    let repo = PostgresRepository::new(&db_url)
        .await
        .expect("Failed to create repository");

    // Create VR first
    let vr = VirtualRouter {
        id: "test_vr".to_string(),
        hostname: "test-host.example.com".to_string(),
        realm: "example.com".to_string(),
        timeout_ms: 3000,
    };
    repo.add_vr(vr).await;

    // Test CREATE routing rule
    let rule = cdde_cms::RoutingRule {
        id: 0, // Will be assigned by database
        vr_id: "test_vr".to_string(),
        priority: 10,
        realm: Some("example.realm".to_string()),
        application_id: Some(16777251),
        destination_host: None,
        target_pool: "pool1".to_string(),
        created_at: None,
    };
    
    let rule_id = repo.create_routing_rule(rule).await;
    assert!(rule_id.is_some(), "Failed to create routing rule");
    let rule_id = rule_id.unwrap();

    // Test READ
    let fetched_rule = repo.get_routing_rule(rule_id).await;
    assert!(fetched_rule.is_some(), "Failed to fetch routing rule");
    let fetched_rule = fetched_rule.unwrap();
    assert_eq!(fetched_rule.priority, 10);
    assert_eq!(fetched_rule.target_pool, "pool1");

    // Test LIST
    let rules = repo.list_routing_rules("test_vr").await;
    assert!(!rules.is_empty(), "Routing rule list should not be empty");

    // Test DELETE
    assert!(repo.delete_routing_rule(rule_id).await, "Failed to delete routing rule");
    
    // Cleanup
    repo.delete_vr("test_vr").await;
}
