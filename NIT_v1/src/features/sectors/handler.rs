use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::{debug, info};

use crate::{error::AppError, state::AppState};

use super::{model::Interest, service};

pub async fn list_interests(State(state): State<AppState>) -> Result<Json<Vec<Interest>>, AppError> {
    debug!(
        target: "nit_v1::sectors::interests",
        event = "interest_list_started",
        "Listing available interests"
    );

    let interests = service::list_interests(&state.db).await?;

    info!(
        target: "nit_v1::sectors::interests",
        event = "interest_list_succeeded",
        interest_count = interests.len(),
        "Listed available interests"
    );

    Ok(Json(interests))
}

pub async fn get_interest(
    State(state): State<AppState>,
    Path(interest_id): Path<i32>,
) -> Result<Json<Interest>, AppError> {
    debug!(
        target: "nit_v1::sectors::interests",
        event = "interest_get_started",
        interest_id,
        "Looking up interest"
    );

    let interest = service::get_interest(&state.db, interest_id).await?;

    info!(
        target: "nit_v1::sectors::interests",
        event = "interest_get_succeeded",
        interest_id,
        "Retrieved interest"
    );

    Ok(Json(interest))
}

pub async fn delete_interest(
    State(state): State<AppState>,
    Path(interest_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    debug!(
        target: "nit_v1::sectors::interests",
        event = "interest_delete_started",
        interest_id,
        "Deleting interest"
    );

    service::delete_interest(&state.db, interest_id).await?;

    info!(
        target: "nit_v1::sectors::interests",
        event = "interest_delete_succeeded",
        interest_id,
        "Deleted interest"
    );

    Ok(StatusCode::NO_CONTENT)
}
