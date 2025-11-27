use crate::db::PostgresRepository;
use crate::error::AppError;
use crate::models::{
    Dictionary, DictionaryAvp, ManipulationRule, PeerConfig, RoutingRule, VirtualRouter,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use tracing::{debug, error};
use utoipa::OpenApi;
use validator::Validate;

use cdde_diameter_dict::DictionaryManager;

/// App state shared across handlers
pub struct AppState {
    pub repository: PostgresRepository,
    pub dictionary_manager: Arc<DictionaryManager>,
}

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        list_vrs,
        create_vr,
        get_vr,
        update_vr,
        delete_vr,
        list_peers,
        create_peer,
        get_peer,
        delete_peer,
        list_dictionaries,
        get_dictionary,
        upload_dictionary,
        delete_dictionary,
        list_routing_rules,
        get_routing_rule,
        create_routing_rule,
        update_routing_rule,
        delete_routing_rule,
        list_manipulation_rules,
        get_manipulation_rule,
        create_manipulation_rule,
        update_manipulation_rule,
        delete_manipulation_rule
    ),
    components(
        schemas(VirtualRouter, PeerConfig, Dictionary, DictionaryAvp, RoutingRule, ManipulationRule)
    ),
    tags(
        (name = "cdde", description = "Cloud Diameter Distribution Engine API")
    )
)]
pub struct ApiDoc;

/// API Router
pub fn create_router(
    repository: PostgresRepository,
    dictionary_manager: Arc<DictionaryManager>,
) -> Router {
    let state = Arc::new(AppState {
        repository,
        dictionary_manager,
    });

    Router::new()
        .route("/api/v1/vrs", get(list_vrs).post(create_vr))
        .route(
            "/api/v1/vrs/:id",
            get(get_vr).put(update_vr).delete(delete_vr),
        )
        .route("/api/v1/peers", get(list_peers).post(create_peer))
        .route(
            "/api/v1/peers/:id",
            get(get_peer).delete(delete_peer).put(update_peer),
        )
        .route(
            "/api/v1/dictionaries",
            get(list_dictionaries).post(upload_dictionary),
        )
        .route(
            "/api/v1/dictionaries/:id",
            get(get_dictionary).delete(delete_dictionary),
        )
        .route(
            "/api/v1/vrs/:vr_id/routing-rules",
            get(list_routing_rules).post(create_routing_rule),
        )
        .route(
            "/api/v1/routing-rules/:id",
            get(get_routing_rule)
                .put(update_routing_rule)
                .delete(delete_routing_rule),
        )
        .route(
            "/api/v1/vrs/:vr_id/manipulation-rules",
            get(list_manipulation_rules).post(create_manipulation_rule),
        )
        .route(
            "/api/v1/manipulation-rules/:id",
            get(get_manipulation_rule)
                .put(update_manipulation_rule)
                .delete(delete_manipulation_rule),
        )
        .with_state(state)
}

// Handlers

#[utoipa::path(
    get,
    path = "/api/v1/vrs",
    responses(
        (status = 200, description = "List all Virtual Routers", body = Vec<VirtualRouter>)
    )
)]
async fn list_vrs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<VirtualRouter>>, AppError> {
    let vrs = state.repository.get_all_vrs().await;
    Ok(Json(vrs))
}

#[utoipa::path(
    post,
    path = "/api/v1/vrs",
    request_body = VirtualRouter,
    responses(
        (status = 201, description = "Virtual Router created"),
        (status = 400, description = "Validation error")
    )
)]
async fn create_vr(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<VirtualRouter>,
) -> Result<StatusCode, AppError> {
    debug!("Creating VR with payload: {:?}", payload);
    // Generate ID if not provided
    if payload.id.is_empty() {
        payload.id = uuid::Uuid::new_v4().to_string();
    }
    payload.validate()?;
    state.repository.add_vr(payload).await;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    get,
    path = "/api/v1/vrs/{id}",
    params(
        ("id" = String, Path, description = "Virtual Router ID")
    ),
    responses(
        (status = 200, description = "Virtual Router found", body = VirtualRouter),
        (status = 404, description = "Virtual Router not found")
    )
)]
async fn get_vr(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<VirtualRouter>, AppError> {
    match state.repository.get_vr(&id).await {
        Some(vr) => Ok(Json(vr)),
        None => Err(AppError::NotFound),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/vrs/{id}",
    params(
        ("id" = String, Path, description = "Virtual Router ID")
    ),
    responses(
        (status = 204, description = "Virtual Router deleted"),
        (status = 404, description = "Virtual Router not found")
    )
)]
async fn delete_vr(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    if state.repository.delete_vr(&id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/vrs/{id}",
    params(
        ("id" = String, Path, description = "Virtual Router ID")
    ),
    request_body = VirtualRouter,
    responses(
        (status = 200, description = "Virtual Router updated"),
        (status = 404, description = "Virtual Router not found"),
        (status = 400, description = "Validation error")
    )
)]
async fn update_vr(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<VirtualRouter>,
) -> Result<StatusCode, AppError> {
    debug!("Updating VR {} with payload: {:?}", id, payload);
    payload.id = id;
    payload.validate()?;

    if state.repository.update_vr(payload).await {
        Ok(StatusCode::OK)
    } else {
        Err(AppError::NotFound)
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/peers",
    responses(
        (status = 200, description = "List all Peers", body = Vec<PeerConfig>)
    )
)]
async fn list_peers(State(state): State<Arc<AppState>>) -> Result<Json<Vec<PeerConfig>>, AppError> {
    let peers = state.repository.get_all_peers().await;
    Ok(Json(peers))
}

#[utoipa::path(
    post,
    path = "/api/v1/peers",
    request_body = PeerConfig,
    responses(
        (status = 201, description = "Peer created"),
        (status = 400, description = "Validation error")
    )
)]
async fn create_peer(
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<PeerConfig>,
) -> Result<StatusCode, AppError> {
    debug!("Creating Peer with payload: {:?}", payload);
    // Generate ID if not provided
    if payload.id.is_empty() {
        payload.id = uuid::Uuid::new_v4().to_string();
    }
    payload.validate()?;
    state.repository.add_peer(payload).await;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    get,
    path = "/api/v1/peers/{hostname}",
    params(
        ("hostname" = String, Path, description = "Peer hostname")
    ),
    responses(
        (status = 200, description = "Peer found", body = PeerConfig),
        (status = 404, description = "Peer not found")
    )
)]
async fn get_peer(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PeerConfig>, AppError> {
    match state.repository.get_peer(&id).await {
        Some(peer) => Ok(Json(peer)),
        None => Err(AppError::NotFound),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/peers/{hostname}",
    params(
        ("hostname" = String, Path, description = "Peer hostname")
    ),
    responses(
        (status = 204, description = "Peer deleted"),
        (status = 404, description = "Peer not found")
    )
)]
async fn delete_peer(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    if state.repository.delete_peer(&id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/peers/{id}",
    params(
        ("id" = String, Path, description = "Peer ID")
    ),
    request_body = PeerConfig,
    responses(
        (status = 200, description = "Peer updated"),
        (status = 404, description = "Peer not found"),
        (status = 400, description = "Validation error")
    )
)]
async fn update_peer(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<PeerConfig>,
) -> Result<StatusCode, AppError> {
    debug!("Updating Peer {} with payload: {:?}", id, payload);
    payload.id = id;
    payload.validate()?;

    if state.repository.update_peer(payload).await {
        Ok(StatusCode::OK)
    } else {
        Err(AppError::NotFound)
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/dictionaries",
    responses(
        (status = 200, description = "List all Dictionaries", body = Vec<Dictionary>)
    )
)]
async fn list_dictionaries(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Dictionary>>, AppError> {
    let dictionaries = state.repository.list_dictionaries().await;
    Ok(Json(dictionaries))
}

#[utoipa::path(
    get,
    path = "/api/v1/dictionaries/{id}",
    params(
        ("id" = i32, Path, description = "Dictionary ID")
    ),
    responses(
        (status = 200, description = "Dictionary found", body = Dictionary),
        (status = 404, description = "Dictionary not found")
    )
)]
async fn get_dictionary(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Dictionary>, AppError> {
    match state.repository.get_dictionary(id).await {
        Some(dict) => Ok(Json(dict)),
        None => Err(AppError::NotFound),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/dictionaries",
    request_body = String,
    responses(
        (status = 201, description = "Dictionary uploaded"),
        (status = 400, description = "Invalid dictionary XML"),
        (status = 500, description = "Internal server error")
    )
)]
async fn upload_dictionary(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    // Parse XML to extract name and version
    // For now, use simple defaults
    let name = format!("dictionary_{}", chrono::Utc::now().timestamp());
    let version = "1.0".to_string();

    // Try to load into dictionary manager first
    match state.dictionary_manager.load_dynamic_dictionary(&body) {
        Ok(_) => {
            // Save to database
            match state.repository.save_dictionary(name, version, body).await {
                Some(id) => Ok((StatusCode::CREATED, Json(serde_json::json!({"id": id})))),
                None => Err(AppError::Internal("Failed to save dictionary".to_string())),
            }
        }
        Err(e) => {
            error!("Failed to load dictionary: {}", e);
            Err(AppError::BadRequest(e.to_string()))
        }
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/dictionaries/{id}",
    params(
        ("id" = i32, Path, description = "Dictionary ID")
    ),
    responses(
        (status = 204, description = "Dictionary deleted"),
        (status = 404, description = "Dictionary not found")
    )
)]
async fn delete_dictionary(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    if state.repository.delete_dictionary(id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

// Routing rule handlers
#[utoipa::path(
    get,
    path = "/api/v1/vrs/{vr_id}/routing-rules",
    params(
        ("vr_id" = String, Path, description = "Virtual Router ID")
    ),
    responses(
        (status = 200, description = "List routing rules for VR", body = Vec<RoutingRule>)
    )
)]
async fn list_routing_rules(
    Path(vr_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<RoutingRule>>, AppError> {
    let rules = state.repository.list_routing_rules(&vr_id).await;
    Ok(Json(rules))
}

#[utoipa::path(
    get,
    path = "/api/v1/routing-rules/{id}",
    params(
        ("id" = i32, Path, description = "Routing Rule ID")
    ),
    responses(
        (status = 200, description = "Routing Rule found", body = RoutingRule),
        (status = 404, description = "Routing Rule not found")
    )
)]
async fn get_routing_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<RoutingRule>, AppError> {
    match state.repository.get_routing_rule(id).await {
        Some(rule) => Ok(Json(rule)),
        None => Err(AppError::NotFound),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/vrs/{vr_id}/routing-rules",
    params(
        ("vr_id" = String, Path, description = "Virtual Router ID")
    ),
    request_body = RoutingRule,
    responses(
        (status = 201, description = "Routing Rule created"),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal server error")
    )
)]
async fn create_routing_rule(
    Path(vr_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<RoutingRule>,
) -> Result<impl IntoResponse, AppError> {
    debug!(
        "Creating Routing Rule for VR {} with payload: {:?}",
        vr_id, payload
    );
    // Ensure the VR ID in the path matches the payload
    payload.vr_id = vr_id;
    payload.validate()?;

    match state.repository.create_routing_rule(payload).await {
        Some(id) => Ok((StatusCode::CREATED, Json(serde_json::json!({"id": id})))),
        None => Err(AppError::Internal(
            "Failed to create routing rule".to_string(),
        )),
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/routing-rules/{id}",
    params(
        ("id" = i32, Path, description = "Routing Rule ID")
    ),
    request_body = RoutingRule,
    responses(
        (status = 200, description = "Routing Rule updated"),
        (status = 404, description = "Routing Rule not found"),
        (status = 400, description = "Validation error")
    )
)]
async fn update_routing_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<RoutingRule>,
) -> Result<StatusCode, AppError> {
    debug!("Updating Routing Rule {} with payload: {:?}", id, payload);
    payload.id = id;
    payload.validate()?;

    if state.repository.update_routing_rule(payload).await {
        Ok(StatusCode::OK)
    } else {
        Err(AppError::NotFound)
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/routing-rules/{id}",
    params(
        ("id" = i32, Path, description = "Routing Rule ID")
    ),
    responses(
        (status = 204, description = "Routing Rule deleted"),
        (status = 404, description = "Routing Rule not found")
    )
)]
async fn delete_routing_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    if state.repository.delete_routing_rule(id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

// Manipulation rule handlers
#[utoipa::path(
    get,
    path = "/api/v1/vrs/{vr_id}/manipulation-rules",
    params(
        ("vr_id" = String, Path, description = "Virtual Router ID")
    ),
    responses(
        (status = 200, description = "List manipulation rules for VR", body = Vec<ManipulationRule>)
    )
)]
async fn list_manipulation_rules(
    Path(vr_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ManipulationRule>>, AppError> {
    let rules = state.repository.list_manipulation_rules(&vr_id).await;
    Ok(Json(rules))
}

#[utoipa::path(
    get,
    path = "/api/v1/manipulation-rules/{id}",
    params(
        ("id" = i32, Path, description = "Manipulation Rule ID")
    ),
    responses(
        (status = 200, description = "Manipulation Rule found", body = ManipulationRule),
        (status = 404, description = "Manipulation Rule not found")
    )
)]
async fn get_manipulation_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ManipulationRule>, AppError> {
    match state.repository.get_manipulation_rule(id).await {
        Some(rule) => Ok(Json(rule)),
        None => Err(AppError::NotFound),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/vrs/{vr_id}/manipulation-rules",
    params(
        ("vr_id" = String, Path, description = "Virtual Router ID")
    ),
    request_body = ManipulationRule,
    responses(
        (status = 201, description = "Manipulation Rule created"),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal server error")
    )
)]
async fn create_manipulation_rule(
    Path(vr_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<ManipulationRule>,
) -> Result<impl IntoResponse, AppError> {
    debug!(
        "Creating Manipulation Rule for VR {} with payload: {:?}",
        vr_id, payload
    );
    payload.vr_id = vr_id;
    payload.validate()?;

    match state.repository.create_manipulation_rule(payload).await {
        Some(id) => Ok((StatusCode::CREATED, Json(serde_json::json!({"id": id})))),
        None => Err(AppError::Internal(
            "Failed to create manipulation rule".to_string(),
        )),
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/manipulation-rules/{id}",
    params(
        ("id" = i32, Path, description = "Manipulation Rule ID")
    ),
    request_body = ManipulationRule,
    responses(
        (status = 200, description = "Manipulation Rule updated"),
        (status = 404, description = "Manipulation Rule not found"),
        (status = 400, description = "Validation error")
    )
)]
async fn update_manipulation_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
    Json(mut payload): Json<ManipulationRule>,
) -> Result<StatusCode, AppError> {
    debug!(
        "Updating Manipulation Rule {} with payload: {:?}",
        id, payload
    );
    payload.id = id;
    payload.validate()?;

    if state.repository.update_manipulation_rule(payload).await {
        Ok(StatusCode::OK)
    } else {
        Err(AppError::NotFound)
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/manipulation-rules/{id}",
    params(
        ("id" = i32, Path, description = "Manipulation Rule ID")
    ),
    responses(
        (status = 204, description = "Manipulation Rule deleted"),
        (status = 404, description = "Manipulation Rule not found")
    )
)]
async fn delete_manipulation_rule(
    Path(id): Path<i32>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    if state.repository.delete_manipulation_rule(id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
