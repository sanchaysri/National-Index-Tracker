use axum::{Router, routing::post};

use super::handler;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/indices", post(handler::create_index))
}
