use axum::{Json, extract::State, http::StatusCode};

use crate::{error::AppError, state::AppState};

use super::{
    model::{DataResponse, RegisterRequest, RegisterResponse},
    service,
};

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<DataResponse<RegisterResponse>>), AppError> {
    let user = service::register_user(&state.db, request).await?;

    Ok((StatusCode::CREATED, Json(DataResponse { data: user })))
}
