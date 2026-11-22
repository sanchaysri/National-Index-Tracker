use super::model::{UserAuthRecord, UserRecord};
use sqlx::PgPool;
use tracing::trace;

// Insert a new registered user into PostgreSQL
pub async fn insert_user(
    pool: &PgPool,
    name: &str,
    email: &str,
    password_hash: &str,
) -> Result<UserRecord, sqlx::Error> {
    // TRACE: Registration database query started
    trace!(
        event = "registration_db_query_started",
        table = "users",
        operation = "INSERT",
        "Executing user registration database query"
    );

    sqlx::query_as::<_, UserRecord>(
        r#"
        INSERT INTO users (name, email, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, name, email
        "#,
    )
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

// Find a user by email for login
pub async fn find_user_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<UserAuthRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserAuthRecord>(
        r#"
        SELECT
            id,
            name,
            email,
            password_hash,
            is_admin
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
}
