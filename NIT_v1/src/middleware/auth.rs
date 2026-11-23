use axum::{
    extract::{Request, State},
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};

use crate::{error::AppError, features::auth::repository, state::AppState};

pub(crate) fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let value = values.next()?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim_start_matches(' ');

    if values.next().is_some()
        || !scheme.eq_ignore_ascii_case("Bearer")
        || token.len() != 64
        || !token.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }

    Some(token)
}

pub(crate) async fn require_access_token(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = bearer_token(request.headers()).ok_or(AppError::InvalidAccessToken)?;
    let token_hash = Sha256::digest(token.as_bytes());
    let valid = repository::has_active_access_token(&state.db, &token_hash)
        .await
        .map_err(|_| {
            tracing::error!(
                event = "access_token_lookup_failed",
                "Could not validate access token"
            );
            AppError::Internal
        })?;

    if !valid {
        return Err(AppError::InvalidAccessToken);
    }

    Ok(next.run(request).await)
}
