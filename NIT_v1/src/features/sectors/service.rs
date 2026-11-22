use sqlx::PgPool;
use tracing::error;

use crate::error::AppError;

use super::{model::Interest, repository};

pub async fn list_interests(pool: &PgPool) -> Result<Vec<Interest>, AppError> {
    repository::list_interests(pool).await.map_err(|_| {
        error!(
            target: "nit_v1::sectors::interests",
            event = "interest_list_db_error",
            error_kind = "database_operation_failed",
            "Could not read interests from the database"
        );
        AppError::Internal
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn lists_interests_in_id_order_with_optional_descriptions(pool: PgPool) {
        seed_interests(&pool).await;

        let interests = list_interests(&pool).await.expect("list interests");
        let fields: Vec<_> = interests
            .iter()
            .map(|interest| {
                (
                    interest.id,
                    interest.name.as_str(),
                    interest.description.as_deref(),
                )
            })
            .collect();

        assert_eq!(
            fields,
            [
                (10, "Health", Some("Healthcare indicators")),
                (20, "Energy", None),
            ]
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn returns_no_interests_when_the_table_is_empty(pool: PgPool) {
        let interests = list_interests(&pool).await.expect("list interests");
        assert!(interests.is_empty());
    }

    #[tokio::test]
    async fn returns_an_internal_error_when_the_pool_is_closed() {
        let pool = PgPool::connect_lazy("postgres://user:password@localhost/test_db")
            .expect("create lazy pool");
        pool.close().await;

        assert!(matches!(list_interests(&pool).await, Err(AppError::Internal)));
    }

    async fn seed_interests(pool: &PgPool) {
        sqlx::query(
            "INSERT INTO interests (id, name, description)
             VALUES (20, 'Energy', NULL), (10, 'Health', 'Healthcare indicators')",
        )
        .execute(pool)
        .await
        .expect("seed interests");
    }
}
