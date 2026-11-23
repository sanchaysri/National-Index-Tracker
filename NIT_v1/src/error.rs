use axum::{
    Json,
    http::{HeaderValue, StatusCode, header::WWW_AUTHENTICATE},
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Conflict(String),
    AlreadyLoggedIn,
    Unauthorized,
    InvalidAccessToken,
    AdminRequired,
    InvalidRefreshToken,
    InterestNotFound,
    InterestAlreadyExists,
    InterestInUse,
    IndexNotFound,
    IndexPayloadNotFound,
    IndexAlreadyExists,
    IndexInUse,
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let bearer_challenge = matches!(self, AppError::InvalidAccessToken);
        let (status, code, message) = match self {
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", message),
            AppError::Conflict(message) => (StatusCode::CONFLICT, "EMAIL_ALREADY_EXISTS", message),
            AppError::AlreadyLoggedIn => (
                StatusCode::CONFLICT,
                "ALREADY_LOGGED_IN",
                "Already logged in".to_string(),
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
            AppError::IndexPayloadNotFound => (
                StatusCode::NOT_FOUND,
                "INDEX_PAYLOAD_NOT_FOUND",
                "Index payload not found".to_string(),
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
            AppError::InvalidAccessToken => (
                StatusCode::UNAUTHORIZED,
                "INVALID_ACCESS_TOKEN",
                "A valid access token is required".to_string(),
            ),
            AppError::AdminRequired => (
                StatusCode::FORBIDDEN,
                "ADMIN_REQUIRED",
                "Administrator access is required".to_string(),
            ),
            AppError::InvalidRefreshToken => (
                StatusCode::UNAUTHORIZED,
                "INVALID_REFRESH_TOKEN",
                "A valid refresh token is required".to_string(),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": code,
                "message": message
            }
        }));

        let mut response = (status, body).into_response();
        if bearer_challenge {
            response
                .headers_mut()
                .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}
