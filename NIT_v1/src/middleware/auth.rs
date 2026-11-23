use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, header::AUTHORIZATION, request::Parts},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};

use crate::{error::AppError, features::auth::repository, state::AppState};

#[derive(Clone)]
struct AuthenticatedUser {
    id: i64,
    is_admin: bool,
}

pub struct RequireAdmin;

impl<S: Sync> FromRequestParts<S> for RequireAdmin {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, AppError> {
        let user = parts
            .extensions
            .get::<AuthenticatedUser>()
            .ok_or(AppError::InvalidAccessToken)?;

        if !user.is_admin {
            tracing::debug!(user_id = user.id, "Admin access denied");
            return Err(AppError::AdminRequired);
        }

        Ok(Self)
    }
}

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
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = bearer_token(request.headers()).ok_or(AppError::InvalidAccessToken)?;
    let token_hash = Sha256::digest(token.as_bytes());
    let (id, is_admin) = repository::find_access_token_user(&state.db, &token_hash)
        .await
        .map_err(|_| {
            tracing::error!(
                event = "access_token_lookup_failed",
                "Could not validate access token"
            );
            AppError::Internal
        })?
        .ok_or(AppError::InvalidAccessToken)?;

    request
        .extensions_mut()
        .insert(AuthenticatedUser { id, is_admin });
    Ok(next.run(request).await)
}
