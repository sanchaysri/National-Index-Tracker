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
use crate::{error::AppError, state::AppState};

pub async fn delete_index(
    State(state): State<AppState>,
    Path(index_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    debug!(target: "nit_v1::indices", event = "index_delete_started", index_id, "Deleting index");
    service::delete_index(&state.db, index_id).await?;
    info!(target: "nit_v1::indices", event = "index_delete_succeeded", index_id, "Deleted index");
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_index(
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
