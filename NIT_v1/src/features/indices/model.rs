use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Deserialize)]
pub struct CreateIndexRequest {
    pub interest_id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(FromRow, Serialize)]
pub struct Index {
    pub id: i32,
    pub interest_id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct IndexPayload {
    pub description: Option<String>,
    pub fields: Value,
    pub records: Value,
}
