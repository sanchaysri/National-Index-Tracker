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

pub(crate) async fn has_active_access_token(
    pool: &PgPool,
    token_hash: &[u8],
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM user_sessions
            WHERE access_token_hash = $1
              AND access_expires_at > statement_timestamp()
              AND revoked_at IS NULL
        )
        "#,
    )
    .bind(token_hash)
    .fetch_one(pool)
    .await
}

pub async fn create_session(
    pool: &PgPool,
    user_id: i64,
    access_token_hash: &[u8],
    refresh_token_hash: &[u8],
    access_ttl_seconds: i32,
    refresh_ttl_seconds: i32,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Lock the user before changing sessions, in the same order as logout.
    sqlx::query("UPDATE users SET is_logged_in = TRUE WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        UPDATE user_sessions
        SET revoked_at = statement_timestamp()
        WHERE user_id = $1 AND revoked_at IS NULL
          AND refresh_expires_at <= statement_timestamp()
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let result = sqlx::query(
        r#"
        INSERT INTO user_sessions (
            user_id, access_token_hash, refresh_token_hash,
            created_at, access_expires_at, refresh_expires_at
        )
        VALUES (
            $1, $2, $3, statement_timestamp(),
            statement_timestamp() + $4::integer * INTERVAL '1 second',
            statement_timestamp() + $5::integer * INTERVAL '1 second'
        )
        ON CONFLICT (user_id) WHERE revoked_at IS NULL DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(access_token_hash)
    .bind(refresh_token_hash)
    .bind(access_ttl_seconds)
    .bind(refresh_ttl_seconds)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(false);
    }

    tx.commit().await?;
    Ok(true)
}

pub(super) async fn rotate_session_tokens(
    pool: &PgPool,
    current_refresh_hash: &[u8],
    access_token_hash: &[u8],
    refresh_token_hash: &[u8],
    access_ttl_seconds: i32,
    refresh_ttl_seconds: i32,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE user_sessions
        SET access_token_hash = $2,
            refresh_token_hash = $3,
            access_expires_at = statement_timestamp() + $4::integer * INTERVAL '1 second',
            refresh_expires_at = statement_timestamp() + $5::integer * INTERVAL '1 second'
        WHERE refresh_token_hash = $1 AND revoked_at IS NULL
          AND refresh_expires_at > clock_timestamp()
        "#,
    )
    .bind(current_refresh_hash)
    .bind(access_token_hash)
    .bind(refresh_token_hash)
    .bind(access_ttl_seconds)
    .bind(refresh_ttl_seconds)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub(super) async fn revoke_session_by_access_token(
    pool: &PgPool,
    token_hash: &[u8],
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let user_id: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT users.id
        FROM users JOIN user_sessions ON user_sessions.user_id = users.id
        WHERE access_token_hash = $1 AND revoked_at IS NULL
          AND access_expires_at > clock_timestamp()
        FOR UPDATE OF users
        "#,
    )
    .bind(token_hash)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(user_id) = user_id else {
        tx.rollback().await?;
        return Ok(false);
    };

    // The token may have rotated while we waited for the user or session row.
    let result = sqlx::query(
        r#"
        UPDATE user_sessions
        SET revoked_at = clock_timestamp()
        WHERE user_id = $1 AND access_token_hash = $2 AND revoked_at IS NULL
          AND access_expires_at > clock_timestamp()
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(false);
    }

    sqlx::query("UPDATE users SET is_logged_in = FALSE WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(true)
}
