use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Incoming JSON from registration
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

// User data returned by PostgreSQL
#[derive(Debug, FromRow)]
pub struct UserRecord {
    pub id: i64,
    pub name: String,
    pub email: String,
}

// Public user data returned to client
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserAuthRecord {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub is_admin: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub message: String,
    pub role: String,
    pub redirect_to: String,
}

// Common success response wrapper
#[derive(Debug, Serialize)]
pub struct DataResponse<T> {
    pub data: T,
}

impl From<UserRecord> for RegisterResponse {
    fn from(user: UserRecord) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
        }
    }
}
