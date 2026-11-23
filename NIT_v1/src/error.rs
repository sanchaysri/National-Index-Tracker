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
    InterestNotFound,
    InterestAlreadyExists,
    InterestInUse,
    IndexNotFound,
    IndexAlreadyExists,
    IndexInUse,
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
            AppError::InterestNotFound => (
                StatusCode::NOT_FOUND,
                "INTEREST_NOT_FOUND",
                "Interest not found".to_string(),
            ),
            AppError::InterestAlreadyExists => (
                StatusCode::CONFLICT,
                "INTEREST_ALREADY_EXISTS",
                "An interest with that name already exists".to_string(),
            ),
            AppError::InterestInUse => (
                StatusCode::CONFLICT,
                "INTEREST_IN_USE",
                "Interest cannot be deleted while related records still reference it".to_string(),
            ),
            AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "An unexpected error occurred".to_string(),
            ),
            AppError::IndexNotFound => (
                StatusCode::NOT_FOUND,
                "INDEX_NOT_FOUND",
                "Index not found".to_string(),
            ),
            AppError::IndexAlreadyExists => (
                StatusCode::CONFLICT,
                "INDEX_ALREADY_EXISTS",
                "An index with that name already exists for this interest".to_string(),
            ),
            AppError::IndexInUse => (
                StatusCode::CONFLICT,
                "INDEX_IN_USE",
                "Related records prevent index deletion".to_string(),
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
