use sqlx::PgPool;
use tracing::{error, warn};

use crate::error::AppError;

use super::{
    model::{CreateInterestRequest, Interest},
    repository,
};

pub async fn create_interest(
    pool: &PgPool,
    request: CreateInterestRequest,
) -> Result<Interest, AppError> {
    let name = request.name.trim();
    if name.is_empty() {
        warn!(
            target: "nit_v1::sectors::interests",
            event = "interest_create_validation_failed",
            field = "name",
            "Interest name is required"
        );
        return Err(AppError::BadRequest("Interest name is required".into()));
    }

    let interest = repository::create_interest(pool, name, request.description.as_deref())
        .await
        .map_err(|_| interest_db_error("create", None))?;

    interest.ok_or_else(|| {
        warn!(
            target: "nit_v1::sectors::interests",
            event = "interest_create_conflict",
            "Interest name already exists"
        );
        AppError::InterestAlreadyExists
    })
}

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
        .map_err(|_| interest_db_error("get", Some(interest_id)))?
        .ok_or_else(|| interest_not_found("get", interest_id))
}

pub async fn delete_interest(pool: &PgPool, interest_id: i32) -> Result<(), AppError> {
    let deleted = repository::delete_interest(pool, interest_id)
        .await
        .map_err(|error| match error.as_database_error() {
            Some(error) if error.is_foreign_key_violation() => {
                warn!(
                    target: "nit_v1::sectors::interests",
                    event = "interest_delete_conflict",
                    interest_id,
                    "Related records prevent interest deletion"
                );
                AppError::InterestInUse
            }
            _ => interest_db_error("delete", Some(interest_id)),
        })?;

    if !deleted {
        return Err(interest_not_found("delete", interest_id));
    }

    Ok(())
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

fn interest_db_error(operation: &'static str, interest_id: Option<i32>) -> AppError {
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
    async fn creates_interest_with_a_generated_id_and_description(pool: PgPool) {
        let created = create_interest(
            &pool,
            CreateInterestRequest {
                name: "  Renewable Energy  ".into(),
                description: Some("Solar and wind indicators".into()),
            },
        )
        .await
        .expect("create interest");

        assert!(created.id > 0);
        let stored = get_interest(&pool, created.id).await.expect("get interest");
        assert_eq!(stored.name, "Renewable Energy");
        assert_eq!(
            stored.description.as_deref(),
            Some("Solar and wind indicators")
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn accepts_a_missing_description_and_rejects_a_duplicate_name(pool: PgPool) {
        let created = create_interest(
            &pool,
            CreateInterestRequest {
                name: "Energy".into(),
                description: None,
            },
        )
        .await
        .expect("create interest without a description");
        assert!(created.description.is_none());

        let duplicate = create_interest(
            &pool,
            CreateInterestRequest {
                name: "Energy".into(),
                description: Some("Duplicate".into()),
            },
        )
        .await;
        assert!(matches!(duplicate, Err(AppError::InterestAlreadyExists)));

        let stored = list_interests(&pool).await.expect("list interests");
        assert_eq!(stored.len(), 1);
        assert!(stored[0].description.is_none());
    }

    #[tokio::test]
    async fn rejects_a_blank_interest_name() {
        let pool = PgPool::connect_lazy("postgres://user:password@localhost/test_db")
            .expect("create lazy pool");
        pool.close().await;

        let result = create_interest(
            &pool,
            CreateInterestRequest {
                name: "  ".into(),
                description: None,
            },
        )
        .await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

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

    #[sqlx::test(migrations = "./migrations")]
    async fn deletes_only_the_requested_interest(pool: PgPool) {
        seed_interests(&pool).await;

        delete_interest(&pool, 10).await.expect("delete interest");

        let interests = list_interests(&pool)
            .await
            .expect("list remaining interests");
        assert_eq!(interests.len(), 1);
        assert_eq!(interests[0].id, 20);
        assert!(matches!(
            delete_interest(&pool, 10).await,
            Err(AppError::InterestNotFound)
        ));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn preserves_interest_when_index_values_prevent_deletion(pool: PgPool) {
        seed_interests(&pool).await;

        sqlx::query("INSERT INTO indices (id, interest_id, name) VALUES (1, 10, 'Admissions')")
            .execute(&pool)
            .await
            .expect("seed related index");
        sqlx::query(
            "INSERT INTO data_sources (id, name, url) VALUES (1, 'Test', 'https://example.com')",
        )
        .execute(&pool)
        .await
        .expect("seed data source");
        sqlx::query(
            "INSERT INTO index_values (index_id, source_id, value, observed_on)
             VALUES (1, 1, 100, DATE '2026-01-01')",
        )
        .execute(&pool)
        .await
        .expect("seed index value");

        assert!(matches!(
            delete_interest(&pool, 10).await,
            Err(AppError::InterestInUse)
        ));
        assert!(get_interest(&pool, 10).await.is_ok());
    }

    #[tokio::test]
    async fn returns_an_internal_error_when_the_pool_is_closed() {
        let pool = PgPool::connect_lazy("postgres://user:password@localhost/test_db")
            .expect("create lazy pool");
        pool.close().await;

        assert!(matches!(list_interests(&pool).await, Err(AppError::Internal)));
        assert!(matches!(
            create_interest(
                &pool,
                CreateInterestRequest {
                    name: "Energy".into(),
                    description: None,
                },
            )
            .await,
            Err(AppError::Internal)
        ));
        assert!(matches!(
            get_interest(&pool, 10).await,
            Err(AppError::Internal)
        ));
        assert!(matches!(
            delete_interest(&pool, 10).await,
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
