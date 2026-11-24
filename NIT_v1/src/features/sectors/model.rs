use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Deserialize)]
pub struct CreateInterestRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(FromRow, Serialize)]
pub struct Interest {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}
