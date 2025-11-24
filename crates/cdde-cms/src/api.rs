use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Router, routing::get,
};
use std::sync::Arc;
use crate::db::PostgresRepository;
use crate::repository::{VirtualRouter, PeerConfig};
use tracing::error;

use cdde_diameter_dict::DictionaryManager;

/// App state shared across handlers
pub struct AppState {
    pub repository: PostgresRepository,
    pub dictionary_manager: Arc<DictionaryManager>,
}

/// API Router
pub fn create_router(repository: PostgresRepository, dictionary_manager: Arc<DictionaryManager>) -> Router {
    let state = Arc::new(AppState { repository, dictionary_manager });

    Router::new()
        .route("/api/v1/vrs", get(list_vrs).post(create_vr))
        .route("/api/v1/vrs/:id", get(get_vr).put(update_vr).delete(delete_vr))
        .route("/api/v1/peers", get(list_peers).post(create_peer))
        .route("/api/v1/peers/:hostname", get(get_peer).delete(delete_peer))
        .route("/api/v1/dictionaries", get(list_dictionaries).post(upload_dictionary))
        .route("/api/v1/dictionaries/:id", get(get_dictionary).delete(delete_dictionary))
        .route("/api/v1/vrs/:vr_id/routing-rules", get(list_routing_rules).post(create_routing_rule))
        .route("/api/v1/routing-rules/:id", get(get_routing_rule).put(update_routing_rule).delete(delete_routing_rule))
        .route("/api/v1/vrs/:vr_id/manipulation-rules", get(list_manipulation_rules).post(create_manipulation_rule))
        .route("/api/v1/manipulation-rules/:id", get(get_manipulation_rule).put(update_manipulation_rule).delete(delete_manipulation_rule))
        .with_state(state)
}

// Handlers

async fn list_vrs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let vrs = state.repository.get_all_vrs().await;
    Json(vrs)
}

async fn create_vr(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VirtualRouter>,
) -> impl IntoResponse {
    state.repository.add_vr(payload).await;
    StatusCode::CREATED
}

async fn get_vr(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.repository.get_vr(&id).await {
        Some(vr) => (StatusCode::OK, Json(Some(vr))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(None::<VirtualRouter>)).into_response(),
    }
}

async fn delete_vr(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.repository.delete_vr(&id).await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn update_vr(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<VirtualRouter>,
) -> impl IntoResponse {
    // Ensure the ID in the path matches the payload
    payload.id = id;
    
    if state.repository.update_vr(payload).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn list_peers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let peers = state.repository.get_all_peers().await;
    Json(peers)
}

async fn create_peer(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PeerConfig>,
) -> impl IntoResponse {
    state.repository.add_peer(payload).await;
    StatusCode::CREATED
}

async fn get_peer(
    Path(hostname): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.repository.get_peer(&hostname).await {
        Some(peer) => (StatusCode::OK, Json(Some(peer))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(None::<PeerConfig>)).into_response(),
    }
}

async fn delete_peer(
    Path(hostname): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.repository.delete_peer(&hostname).await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn list_dictionaries(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let dictionaries = state.repository.list_dictionaries().await;
    Json(dictionaries)
}

async fn get_dictionary(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.repository.get_dictionary(id).await {
        Some(dict) => (StatusCode::OK, Json(Some(dict))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(None::<crate::models::Dictionary>)).into_response(),
    }
}

async fn upload_dictionary(
    State(state): State<Arc<AppState>>,
    body: String,
) -> impl IntoResponse {
    // Parse XML to extract name and version
    // For now, use simple defaults
    let name = format!("dictionary_{}", chrono::Utc::now().timestamp());
    let version = "1.0".to_string();
    
    // Try to load into dictionary manager first
    match state.dictionary_manager.load_dynamic_dictionary(&body) {
        Ok(_) => {
            // Save to database
            match state.repository.save_dictionary(name, version, body).await {
                Some(id) => (StatusCode::CREATED, Json(serde_json::json!({"id": id}))).into_response(),
                None => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to save dictionary"}))).into_response(),
            }
        }
        Err(e) => {
            error!("Failed to load dictionary: {}", e);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))).into_response()
        }
    }
}

async fn delete_dictionary(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.repository.delete_dictionary(id).await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

// Routing rule handlers
async fn list_routing_rules(
    Path(vr_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rules = state.repository.list_routing_rules(&vr_id).await;
    Json(rules)
}

async fn get_routing_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.repository.get_routing_rule(id).await {
        Some(rule) => (StatusCode::OK, Json(Some(rule))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(None::<crate::models::RoutingRule>)).into_response(),
    }
}

async fn create_routing_rule(
    Path(vr_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<crate::models::RoutingRule>,
) -> impl IntoResponse {
    // Ensure the VR ID in the path matches the payload
    payload.vr_id = vr_id;
    
    match state.repository.create_routing_rule(payload).await {
        Some(id) => (StatusCode::CREATED, Json(serde_json::json!({"id": id}))).into_response(),
        None => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to create routing rule"}))).into_response(),
    }
}

async fn update_routing_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<crate::models::RoutingRule>,
) -> impl IntoResponse {
    // Ensure the ID in the path matches the payload
    payload.id = id;
    
    if state.repository.update_routing_rule(payload).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn delete_routing_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.repository.delete_routing_rule(id).await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

// Manipulation rule handlers
async fn list_manipulation_rules(
    Path(vr_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let rules = state.repository.list_manipulation_rules(&vr_id).await;
    Json(rules)
}

async fn get_manipulation_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.repository.get_manipulation_rule(id).await {
        Some(rule) => (StatusCode::OK, Json(Some(rule))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(None::<crate::models::ManipulationRule>)).into_response(),
    }
}

async fn create_manipulation_rule(
    Path(vr_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<crate::models::ManipulationRule>,
) -> impl IntoResponse {
    // Ensure the VR ID in the path matches the payload
    payload.vr_id = vr_id;
    
    match state.repository.create_manipulation_rule(payload).await {
        Some(id) => (StatusCode::CREATED, Json(serde_json::json!({"id": id}))).into_response(),
        None => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to create manipulation rule"}))).into_response(),
    }
}

async fn update_manipulation_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<crate::models::ManipulationRule>,
) -> impl IntoResponse {
    // Ensure the ID in the path matches the payload
    payload.id = id;
    
    if state.repository.update_manipulation_rule(payload).await {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn delete_manipulation_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if state.repository.delete_manipulation_rule(id).await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_list_vrs() {
        let repository = ConfigRepository::new();
        repository.add_vr(VirtualRouter {
            id: "vr1".to_string(),
            hostname: "h1".to_string(),
            realm: "r1".to_string(),
            timeout_ms: 1000,
        }).await;

        let app = create_router(repository, Arc::new(DictionaryManager::new()));

        let response = app
            .oneshot(Request::builder().uri("/api/v1/vrs").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_vr() {
        let repository = ConfigRepository::new();
        let app = create_router(repository.clone(), Arc::new(DictionaryManager::new()));

        let vr = VirtualRouter {
            id: "vr2".to_string(),
            hostname: "h2".to_string(),
            realm: "r2".to_string(),
            timeout_ms: 2000,
        };
        let body = serde_json::to_string(&vr).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/vrs")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        assert!(repository.get_vr("vr2").await.is_some());
    }
}
*/
