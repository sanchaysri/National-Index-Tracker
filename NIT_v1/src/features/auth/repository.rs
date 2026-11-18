use sqlx::PgPool;

use super::model::UserRecord;

pub async fn insert_user(
    pool: &PgPool,
    name: &str,
    email: &str,
    password_hash: &str,
) -> Result<UserRecord, sqlx::Error> {
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
