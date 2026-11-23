use sqlx::PgPool;
use tracing::{error, warn};

use super::{
    model::{CreateIndexRequest, Index},
    repository,
};
use crate::error::AppError;

pub async fn create_index(pool: &PgPool, request: CreateIndexRequest) -> Result<Index, AppError> {
    let interest_id = request.interest_id;
    let name = request.name.trim();
    if name.is_empty() {
        warn!(
            target: "nit_v1::indices",
            event = "index_create_invalid_name",
            interest_id,
            "Index name is required"
        );
        return Err(AppError::BadRequest("Index name is required".into()));
    }

    repository::insert_index(
        pool,
        interest_id,
        name,
        request.description.as_deref(),
        request.unit.as_deref(),
    )
    .await
    .map_err(|err| match err.as_database_error() {
        Some(db)
            if db.is_foreign_key_violation() && db.constraint() == Some("fk_indices_interest") =>
        {
            warn!(
                target: "nit_v1::indices",
                event = "index_create_interest_not_found",
                interest_id,
                "Cannot create index for an unknown interest"
            );
            AppError::InterestNotFound
        }
        _ => {
            error!(
                target: "nit_v1::indices",
                event = "index_create_db_error",
                interest_id,
                "Could not insert index"
            );
            AppError::Internal
        }
    })
}

pub async fn delete_index(pool: &PgPool, index_id: i32) -> Result<(), AppError> {
    match repository::delete_index(pool, index_id).await {
        Ok(true) => Ok(()),
        Ok(false) => {
            warn!(target: "nit_v1::indices", event = "index_delete_not_found", index_id, "Index does not exist");
            Err(AppError::IndexNotFound)
        }
        Err(sqlx::Error::Database(err)) if err.is_foreign_key_violation() => {
            warn!(target: "nit_v1::indices", event = "index_delete_conflict", index_id, "Related records prevent index deletion");
            Err(AppError::IndexInUse)
        }
        Err(_) => {
            error!(target: "nit_v1::indices", event = "index_delete_db_error", index_id, "Could not delete index");
            Err(AppError::Internal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(interest_id: i32, name: &str) -> CreateIndexRequest {
        CreateIndexRequest {
            interest_id,
            name: name.into(),
            description: None,
            unit: None,
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn persists_index_for_existing_interest(pool: PgPool) {
        let interest_id =
            sqlx::query_scalar("INSERT INTO interests (name) VALUES ('Energy') RETURNING id")
                .fetch_one(&pool)
                .await
                .unwrap();
        let mut input = request(interest_id, "  Solar output  ");
        input.description = Some("Daily generation".into());
        input.unit = Some("MWh".into());
        let created = create_index(&pool, input).await.expect("create index");
        let stored = sqlx::query_as::<_, Index>("SELECT * FROM indices WHERE id = $1")
            .bind(created.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored.interest_id, interest_id);
        assert_eq!(stored.name, "Solar output");
        assert_eq!(stored.description.as_deref(), Some("Daily generation"));
        assert_eq!(stored.unit.as_deref(), Some("MWh"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn rejects_unknown_interest_without_inserting(pool: PgPool) {
        let result = create_index(&pool, request(999, "Solar output")).await;
        assert!(matches!(result, Err(AppError::InterestNotFound)));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM indices")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn rejects_blank_name_before_using_pool() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        pool.close().await;
        let result = create_index(&pool, request(1, "  ")).await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn deletes_only_the_requested_index(pool: PgPool) {
        sqlx::query("INSERT INTO interests (id, name) VALUES (10, 'Energy')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO indices (id, interest_id, name) VALUES (1, 10, 'Solar'), (2, 10, 'Wind')",
        )
        .execute(&pool)
        .await
        .unwrap();

        delete_index(&pool, 1).await.expect("delete Solar");
        let remaining: Vec<i32> = sqlx::query_scalar("SELECT id FROM indices")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, [2]);
        assert!(matches!(
            delete_index(&pool, 1).await,
            Err(AppError::IndexNotFound)
        ));
    }
}
