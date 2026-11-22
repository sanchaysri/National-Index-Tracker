use serde::Serialize;
use sqlx::FromRow;

#[derive(FromRow, Serialize)]
pub struct Interest {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}
