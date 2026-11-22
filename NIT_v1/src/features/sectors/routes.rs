use axum::{Router, routing::get};

use crate::state::AppState;

use super::handler;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/interests", get(handler::list_interests))
        .route(
            "/api/v1/interest/{interest_id}",
            get(handler::get_interest),
        )
}
