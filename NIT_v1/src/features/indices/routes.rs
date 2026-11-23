use axum::{
    Router,
    routing::{get, post},
};

use super::handler;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/interests/{interest_id}/indices",
            get(handler::list_for_interest),
        )
        .route("/api/v1/indices", post(handler::create_index))
        .route(
            "/api/v1/indices/{index_id}",
            get(handler::get_index).delete(handler::delete_index),
        )
}
