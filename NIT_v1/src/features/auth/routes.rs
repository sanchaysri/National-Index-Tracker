use axum::{Router, routing::post};

use crate::state::AppState;

use super::handler;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/register", post(handler::register))
        .route("/api/v1/auth/login", post(handler::login))
        .route("/api/v1/auth/logout", post(handler::logout))
}
