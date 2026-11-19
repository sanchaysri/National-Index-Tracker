use axum::{Json, extract::State, http::StatusCode};

use crate::{error::AppError, state::AppState};

use super::{
    model::{DataResponse, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse},
    service,
};

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<DataResponse<RegisterResponse>>), AppError> {
    let user = service::register_user(&state.db, request).await?;

    Ok((StatusCode::CREATED, Json(DataResponse { data: user })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<DataResponse<LoginResponse>>, AppError> {
    let response = service::login_user(&state.db, request).await?;

    Ok(Json(DataResponse { data: response }))
}

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
