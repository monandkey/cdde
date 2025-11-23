use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Router, routing::get,
};
use std::sync::Arc;
use crate::repository::{ConfigRepository, VirtualRouter, PeerConfig};

/// App state shared across handlers
pub struct AppState {
    pub repository: ConfigRepository,
}

/// API Router
pub fn create_router(repository: ConfigRepository) -> Router {
    let state = Arc::new(AppState { repository });

    Router::new()
        .route("/api/v1/vrs", get(list_vrs).post(create_vr))
        .route("/api/v1/vrs/:id", get(get_vr).delete(delete_vr))
        .route("/api/v1/peers", get(list_peers).post(create_peer))
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

        let app = create_router(repository);

        let response = app
            .oneshot(Request::builder().uri("/api/v1/vrs").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_vr() {
        let repository = ConfigRepository::new();
        let app = create_router(repository.clone());

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
