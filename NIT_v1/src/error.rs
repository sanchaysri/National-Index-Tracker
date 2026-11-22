use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Conflict(String),
    AlreadyLoggedIn,
    AlreadyLoggedOut,
    Unauthorized,
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", message),
            AppError::Conflict(message) => (StatusCode::CONFLICT, "EMAIL_ALREADY_EXISTS", message),
            AppError::AlreadyLoggedIn => (
                StatusCode::CONFLICT,
                "ALREADY_LOGGED_IN",
                "Already logged in".to_string(),
            ),
            AppError::AlreadyLoggedOut => (
                StatusCode::CONFLICT,
                "ALREADY_LOGGED_OUT",
                "Already logged out".to_string(),
            ),
            AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "An unexpected error occurred".to_string(),
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                "Invalid email or password".to_string(),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": code,
                "message": message
            }
        }));

        (status, body).into_response()
    }
}
