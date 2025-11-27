use cdde_cms::models::VirtualRouter;
use reqwest::StatusCode;

mod common;

#[tokio::test]
async fn test_vr_lifecycle() {
    let client = common::get_client();
    let base_url = common::get_base_url();

    // 1. Create VR
    let vr_id = uuid::Uuid::new_v4().to_string();
    let vr = VirtualRouter {
        id: vr_id.clone(),
        hostname: format!("test-vr-{}.example.com", vr_id),
        realm: "example.com".to_string(),
        timeout_ms: 3000,
    };

    let res = client
        .post(format!("{}/vrs", base_url))
        .json(&vr)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::CREATED);

    // 2. Get VR
    let res = client
        .get(format!("{}/vrs/{}", base_url, vr_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);
    let fetched_vr: VirtualRouter = res.json().await.expect("Failed to parse JSON");
    assert_eq!(fetched_vr.id, vr_id);
    assert_eq!(fetched_vr.hostname, vr.hostname);

    // 3. Update VR
    let mut updated_vr = fetched_vr.clone();
    updated_vr.hostname = format!("updated-{}.example.com", vr_id);

    let res = client
        .put(format!("{}/vrs/{}", base_url, vr_id))
        .json(&updated_vr)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::OK);

    // Verify update
    let res = client
        .get(format!("{}/vrs/{}", base_url, vr_id))
        .send()
        .await
        .expect("Failed to send request");
    
    let fetched_updated_vr: VirtualRouter = res.json().await.expect("Failed to parse JSON");
    assert_eq!(fetched_updated_vr.hostname, updated_vr.hostname);

    // 4. Delete VR
    let res = client
        .delete(format!("{}/vrs/{}", base_url, vr_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Verify deletion
    let res = client
        .get(format!("{}/vrs/{}", base_url, vr_id))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
