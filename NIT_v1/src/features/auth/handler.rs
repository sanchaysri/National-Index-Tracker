use axum::{Json, extract::State, http::StatusCode};
use tracing::info;

use crate::{error::AppError, state::AppState};

use super::{
    model::{DataResponse, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse},
    service,
};

// ========================================
// REGISTRATION HANDLER
// ========================================

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<DataResponse<RegisterResponse>>), AppError> {
    // INFO: Registration request received
    info!(
        target: "nit_v1::auth::registration",
        event = "registration_started",
        "Registration request received"
    );

    // Call registration service
    let user = service::register_user(&state.db, request).await?;

    // INFO: Registration successful
    info!(
        target: "nit_v1::auth::registration",
        event = "registration_success",
        user_id = user.id,
        "User registered successfully"
    );

    Ok((StatusCode::CREATED, Json(DataResponse { data: user })))
}

// ========================================
// LOGIN HANDLER
// ========================================

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<DataResponse<LoginResponse>>, AppError> {
    // INFO: Login request received
    info!(
        target: "nit_v1::auth::login",
        event = "login_started",
        "Login request received"
    );

    // Call login service
    let response = service::login_user(&state.db, request).await?;

    // INFO: Login successful
    info!(
        target: "nit_v1::auth::login",
        event = "login_success",
        "User logged in successfully"
    );

    Ok(Json(DataResponse { data: response }))
}

// ========================================
// UNIT TESTS
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn test_register_success(pool: sqlx::PgPool) {
        let (status, Json(response)) = register(
            State(AppState { db: pool.clone() }),
            Json(RegisterRequest {
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                password: "securepassword".to_string(),
            }),
        )
        .await
        .expect("registration should succeed");

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.data.name, "Test User");
        assert_eq!(response.data.email, "test@example.com");

        let stored_id: i64 = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind("test@example.com")
            .fetch_one(&pool)
            .await
            .expect("registered user should exist in the test table");

        assert_eq!(response.data.id, stored_id);
    }

    #[tokio::test]
    async fn test_register_email_failure() {
        let mock_state = AppState {
            db: sqlx::PgPool::connect_lazy("postgres://user:password@localhost/test_db").unwrap(),
        };

        let response = register(
            State(mock_state),
            Json(RegisterRequest {
                name: "Test User".to_string(),
                email: "invalid-email".to_string(),
                password: "securepassword".to_string(),
            }),
        )
        .await;

        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_register_name_failure() {
        let mock_state = AppState {
            db: sqlx::PgPool::connect_lazy("postgres://user:password@localhost/test_db").unwrap(),
        };

        let response = register(
            State(mock_state),
            Json(RegisterRequest {
                name: "".to_string(),
                email: "test@example.com".to_string(),
                password: "securepassword".to_string(),
            }),
        )
        .await;

        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_register_password_failure() {
        let mock_state = AppState {
            db: sqlx::PgPool::connect_lazy("postgres://user:password@localhost/test_db").unwrap(),
        };

        let response = register(
            State(mock_state),
            Json(RegisterRequest {
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                password: "".to_string(),
            }),
        )
        .await;

        assert!(response.is_err());
    }
}
