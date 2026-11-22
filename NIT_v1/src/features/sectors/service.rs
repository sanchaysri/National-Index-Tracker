use sqlx::PgPool;
use tracing::{error, warn};

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

pub async fn get_interest(pool: &PgPool, interest_id: i32) -> Result<Interest, AppError> {
    repository::get_interest(pool, interest_id)
        .await
        .map_err(|_| interest_db_error("get", interest_id))?
        .ok_or_else(|| interest_not_found("get", interest_id))
}

fn interest_not_found(operation: &'static str, interest_id: i32) -> AppError {
    warn!(
        target: "nit_v1::sectors::interests",
        event = "interest_not_found",
        operation,
        interest_id,
        "Interest does not exist"
    );
    AppError::InterestNotFound
}

fn interest_db_error(operation: &'static str, interest_id: i32) -> AppError {
    error!(
        target: "nit_v1::sectors::interests",
        event = "interest_db_error",
        error_kind = "database_operation_failed",
        operation,
        interest_id,
        "Interest database operation failed"
    );
    AppError::Internal
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

    #[sqlx::test(migrations = "./migrations")]
    async fn gets_only_the_requested_interest(pool: PgPool) {
        seed_interests(&pool).await;

        let interest = get_interest(&pool, 10).await.expect("get interest");
        assert_eq!(interest.id, 10);
        assert_eq!(interest.name, "Health");
        assert_eq!(interest.description.as_deref(), Some("Healthcare indicators"));
        assert!(matches!(
            get_interest(&pool, 999).await,
            Err(AppError::InterestNotFound)
        ));
    }

    #[tokio::test]
    async fn returns_an_internal_error_when_the_pool_is_closed() {
        let pool = PgPool::connect_lazy("postgres://user:password@localhost/test_db")
            .expect("create lazy pool");
        pool.close().await;

        assert!(matches!(list_interests(&pool).await, Err(AppError::Internal)));
        assert!(matches!(
            get_interest(&pool, 10).await,
            Err(AppError::Internal)
        ));
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
