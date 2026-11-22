use axum::{Json, extract::State};
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
