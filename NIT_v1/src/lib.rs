pub mod config;
pub mod db;
pub mod error;
pub mod features;
pub mod interests;
pub mod middleware;
pub mod state;

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde_json::{Value, json};

use state::AppState;

async fn health(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(Json(json!({
        "status": "ok",
        "database": "connected"
    })))
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/profile",
            get(|| async { axum::response::Html(include_str!("../frontend/profile.html")) }),
        )
        .route("/api/v1/interests", get(interests::list_interests))
        .route(
            "/api/v1/interests/{interest_id}/indices",
            get(interests::list_interest_indices),
        )
        .merge(features::auth::routes::routes())
        .with_state(state)
}
