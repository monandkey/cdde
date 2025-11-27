use cdde_cms::models::{ManipulationRule, VirtualRouter};
use reqwest::StatusCode;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_manipulation_rule_lifecycle() {
    let client = common::get_client();
    let base_url = common::get_base_url();

    // Setup: VR
    let vr_id = uuid::Uuid::new_v4().to_string();
    let vr = VirtualRouter {
        id: vr_id.clone(),
        hostname: format!("vr-mr-{}.example.com", vr_id),
        realm: "example.com".to_string(),
        timeout_ms: 3000,
    };
    client.post(format!("{}/vrs", base_url)).json(&vr).send().await.unwrap();

    // 1. Create Manipulation Rule
    // We omit vr_id in payload to test auto-population from path
    let rule_payload = json!({
        "priority": 10,
        "rule_json": {
            "condition": "avp.Origin-Realm == 'old.example.com'",
            "action": "set avp.Origin-Realm = 'new.example.com'"
        }
    });

    let res = client
        .post(format!("{}/vrs/{}/manipulation-rules", base_url, vr_id))
        .json(&rule_payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::CREATED);
    let body: serde_json::Value = res.json().await.unwrap();
    let rule_id = body["id"].as_i64().expect("ID not found");

    // 2. Get Manipulation Rule
    let res = client
        .get(format!("{}/manipulation-rules/{}", base_url, rule_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);
    let fetched_rule: ManipulationRule = res.json().await.expect("Failed to parse JSON");
    assert_eq!(fetched_rule.vr_id, vr_id);
    assert_eq!(fetched_rule.priority, 10);

    // 3. List Manipulation Rules
    let res = client
        .get(format!("{}/vrs/{}/manipulation-rules", base_url, vr_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);
    let rules: Vec<ManipulationRule> = res.json().await.unwrap();
    assert!(rules.iter().any(|r| r.id as i64 == rule_id));

    // 4. Update Manipulation Rule
    // Note: vr_id is required in body for updates
    let mut updated_payload = json!({
        "vr_id": vr_id,
        "priority": 20,
        "rule_json": {
            "condition": "avp.Origin-Realm == 'old.example.com'",
            "action": "set avp.Origin-Realm = 'updated.example.com'"
        }
    });

    let res = client
        .put(format!("{}/manipulation-rules/{}", base_url, rule_id))
        .json(&updated_payload)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);

    // 5. Delete Manipulation Rule
    let res = client
        .delete(format!("{}/manipulation-rules/{}", base_url, rule_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Cleanup
    client.delete(format!("{}/vrs/{}", base_url, vr_id)).send().await.unwrap();
}
