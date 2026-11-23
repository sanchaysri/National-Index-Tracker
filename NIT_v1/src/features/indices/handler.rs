use axum::{Json, extract::State, http::StatusCode};
use tracing::{debug, info};

use super::{
    model::{CreateIndexRequest, Index},
    service,
};
use crate::{error::AppError, state::AppState};

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
