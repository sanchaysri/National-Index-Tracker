use sqlx::PgPool;

use super::model::Interest;

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
