use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::{debug, info};

use super::{
    model::{CreateIndexRequest, Index},
    service,
};
use crate::{error::AppError, middleware::auth::RequireAdmin, state::AppState};

pub async fn get_index(
    State(state): State<AppState>,
    Path(index_id): Path<i32>,
) -> Result<Json<Index>, AppError> {
    debug!(target: "nit_v1::indices", event = "index_get_started", index_id, "Fetching index");
    let index = service::get_index(&state.db, index_id).await?;
    info!(target: "nit_v1::indices", event = "index_get_succeeded", index_id, "Fetched index");
    Ok(Json(index))
}

pub async fn get_payload(
    State(state): State<AppState>,
    Path(index_id): Path<i32>,
) -> Result<Json<super::model::IndexPayload>, AppError> {
    debug!(
        target: "nit_v1::indices",
        event = "index_payload_get_started",
        index_id,
        "Fetching index payload"
    );
    let payload = service::get_latest_payload(&state.db, index_id).await?;
    info!(
        target: "nit_v1::indices",
        event = "index_payload_get_succeeded",
        index_id,
        "Fetched index payload"
    );
    Ok(Json(payload))
}

pub async fn list_for_interest(
    State(state): State<AppState>,
    Path(interest_id): Path<i32>,
) -> Result<Json<Vec<Index>>, AppError> {
    debug!(
        target: "nit_v1::indices",
        event = "interest_indices_list_started",
        interest_id,
        "Listing indices for interest"
    );
    let indices = service::list_for_interest(&state.db, interest_id).await?;
    info!(
        target: "nit_v1::indices",
        event = "interest_indices_list_succeeded",
        interest_id,
        index_count = indices.len(),
        "Listed indices for interest"
    );
    Ok(Json(indices))
}

pub async fn delete_index(
    _: RequireAdmin,
    State(state): State<AppState>,
    Path(index_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    debug!(target: "nit_v1::indices", event = "index_delete_started", index_id, "Deleting index");
    service::delete_index(&state.db, index_id).await?;
    info!(target: "nit_v1::indices", event = "index_delete_succeeded", index_id, "Deleted index");
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_index(
    _: RequireAdmin,
    State(state): State<AppState>,
    Json(request): Json<CreateIndexRequest>,
) -> Result<(StatusCode, Json<Index>), AppError> {
    debug!(
        target: "nit_v1::indices",
        event = "index_create_started",
        interest_id = request.interest_id,
        "Creating index"
    );
    let index = service::create_index(&state.db, request).await?;
    info!(
        target: "nit_v1::indices",
        event = "index_create_succeeded",
        index_id = index.id,
        interest_id = index.interest_id,
        "Created index"
    );
    Ok((StatusCode::CREATED, Json(index)))
}
