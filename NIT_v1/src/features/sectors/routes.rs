use axum::{Router, routing::get};

use crate::state::AppState;

use super::handler;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/interests",
            get(handler::list_interests).post(handler::create_interest),
        )
        .route(
            "/api/v1/interest/{interest_id}",
            get(handler::get_interest).delete(handler::delete_interest),
        )
}
