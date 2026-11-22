use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;
use sqlx::Row;

use crate::state::AppState;

#[derive(Serialize)]
pub struct Interest {
    id: i64,
    name: String,
}

#[derive(Serialize)]
pub struct IndexValue {
    id: i64,
    name: String,
    value: String,
    unit: Option<String>,
    period: Option<String>,
}

pub async fn list_interests(
    State(state): State<AppState>,
) -> Result<Json<Vec<Interest>>, StatusCode> {
    let rows = sqlx::query("SELECT id, name FROM interests ORDER BY id")
        .fetch_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let interests = rows
        .into_iter()
        .map(|row| Interest {
            id: row.get("id"),
            name: row.get("name"),
        })
        .collect();

    Ok(Json(interests))
}

pub async fn list_interest_indices(
    State(state): State<AppState>,
    Path(interest_id): Path<i64>,
) -> Result<Json<Vec<IndexValue>>, StatusCode> {
    let rows = sqlx::query(
        "SELECT indices.id, indices.name, index_values.value::text AS value,
                index_values.unit, index_values.period
         FROM indices
         JOIN index_values ON index_values.index_id = indices.id
         WHERE index_values.interest_id = $1
         ORDER BY indices.id",
    )
    .bind(interest_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let values = rows
        .into_iter()
        .map(|row| IndexValue {
            id: row.get("id"),
            name: row.get("name"),
            value: row.get("value"),
            unit: row.get("unit"),
            period: row.get("period"),
        })
        .collect();

    Ok(Json(values))
}
