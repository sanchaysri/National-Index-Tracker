use sqlx::PgPool;

use super::model::Index;

pub async fn list_for_interest(pool: &PgPool, interest_id: i32) -> Result<Vec<Index>, sqlx::Error> {
    sqlx::query_as::<_, Index>(
        "SELECT id, interest_id, name, description, unit FROM indices
         WHERE interest_id = $1 ORDER BY id",
    )
    .bind(interest_id)
    .fetch_all(pool)
    .await
}

pub async fn delete_index(pool: &PgPool, index_id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM indices WHERE id = $1")
        .bind(index_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn insert_index(
    pool: &PgPool,
    interest_id: i32,
    name: &str,
    description: Option<&str>,
    unit: Option<&str>,
) -> Result<Option<Index>, sqlx::Error> {
    sqlx::query_as::<_, Index>(
        "INSERT INTO indices (interest_id, name, description, unit)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT ON CONSTRAINT indices_interest_id_name_key DO NOTHING
         RETURNING id, interest_id, name, description, unit",
    )
    .bind(interest_id)
    .bind(name)
    .bind(description)
    .bind(unit)
    .fetch_optional(pool)
    .await
}
