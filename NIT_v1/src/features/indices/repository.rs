use sqlx::PgPool;

use super::model::Index;

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
) -> Result<Index, sqlx::Error> {
    sqlx::query_as::<_, Index>(
        "INSERT INTO indices (interest_id, name, description, unit)
         VALUES ($1, $2, $3, $4)
         RETURNING id, interest_id, name, description, unit",
    )
    .bind(interest_id)
    .bind(name)
    .bind(description)
    .bind(unit)
    .fetch_one(pool)
    .await
}
