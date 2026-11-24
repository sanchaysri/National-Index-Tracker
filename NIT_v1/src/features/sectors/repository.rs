use sqlx::PgPool;

use super::model::Interest;

pub async fn create_interest(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
) -> Result<Option<Interest>, sqlx::Error> {
    sqlx::query_as::<_, Interest>(
        "INSERT INTO interests (name, description)
         VALUES ($1, $2)
         ON CONFLICT (name) DO NOTHING
         RETURNING id, name, description",
    )
    .bind(name)
    .bind(description)
    .fetch_optional(pool)
    .await
}

pub async fn list_interests(pool: &PgPool) -> Result<Vec<Interest>, sqlx::Error> {
    sqlx::query_as::<_, Interest>("SELECT id, name, description FROM interests ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn get_interest(pool: &PgPool, interest_id: i32) -> Result<Option<Interest>, sqlx::Error> {
    sqlx::query_as::<_, Interest>("SELECT id, name, description FROM interests WHERE id = $1")
        .bind(interest_id)
        .fetch_optional(pool)
        .await
}

pub async fn delete_interest(pool: &PgPool, interest_id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM interests WHERE id = $1")
        .bind(interest_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
