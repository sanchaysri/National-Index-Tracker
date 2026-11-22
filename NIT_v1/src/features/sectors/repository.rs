use sqlx::PgPool;

use super::model::Interest;

pub async fn list_interests(pool: &PgPool) -> Result<Vec<Interest>, sqlx::Error> {
    sqlx::query_as::<_, Interest>("SELECT id, name, description FROM interests ORDER BY id")
        .fetch_all(pool)
        .await
}
