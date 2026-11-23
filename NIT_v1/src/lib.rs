pub mod config;
pub mod db;
pub mod error;
pub mod features;
pub mod interests;
pub mod middleware;
pub mod state;

use axum::{
    Json, Router,
    extract::{MatchedPath, Request, State},
    http::StatusCode,
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use serde_json::{Value, json};
use std::time::Instant;

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
    let protected_routes = Router::new()
        .merge(features::indices::routes::routes())
        .merge(features::sectors::routes::routes())
        .route_layer(from_fn_with_state(
            state.clone(),
            middleware::auth::require_access_token,
        ));

    Router::new()
        .route("/api/v1/health", get(health))
        .merge(features::auth::routes::routes())
        .merge(protected_routes)
        .layer(axum::middleware::from_fn(log_request))
        .with_state(state)
}

async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let route = request.extensions().get::<MatchedPath>().cloned();
    let started_at = Instant::now();
    let response = next.run(request).await;

    tracing::info!(
        event = "http_request_completed",
        %method,
        route = route.as_ref().map_or("<unmatched>", MatchedPath::as_str),
        status = response.status().as_u16(),
        duration_ms = started_at.elapsed().as_secs_f64() * 1_000.0,
        "HTTP request completed"
    );

    response
}
