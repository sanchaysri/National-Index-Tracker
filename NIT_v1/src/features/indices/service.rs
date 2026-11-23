use serde_json::{Value, json};
use sqlx::PgPool;
use tracing::{error, warn};

use super::{
    model::{CreateIndexRequest, Index, IndexPayload},
    repository,
};
use crate::{error::AppError, features::sectors};

pub async fn get_index(pool: &PgPool, index_id: i32) -> Result<Index, AppError> {
    match repository::get_index(pool, index_id).await {
        Ok(Some(index)) => Ok(index),
        Ok(None) => {
            warn!(target: "nit_v1::indices", event = "index_get_not_found", index_id, "Index does not exist");
            Err(AppError::IndexNotFound)
        }
        Err(err) => {
            error!(
                target: "nit_v1::indices",
                event = "index_get_db_error",
                index_id,
                error = %err,
                "Could not fetch index"
            );
            Err(AppError::Internal)
        }
    }
}

pub async fn get_latest_payload(pool: &PgPool, index_id: i32) -> Result<IndexPayload, AppError> {
    get_index(pool, index_id).await?;
    let (description, payload) = repository::get_latest_payload(pool, index_id)
        .await
        .map_err(|err| {
            error!(
                target: "nit_v1::indices",
                event = "index_payload_get_db_error",
                index_id,
                error = %err,
                "Could not fetch index payload"
            );
            AppError::Internal
        })?
        .ok_or(AppError::IndexPayloadNotFound)?;

    let (description, fields) = split_description_fields(description)?;
    Ok(IndexPayload {
        description,
        fields,
        records: payload.0,
    })
}

fn split_description_fields(
    description: Option<String>,
) -> Result<(Option<String>, Value), AppError> {
    const MARKER: &str = "[data.gov.in fields source_id=";
    let Some(description) = description else {
        return Ok((None, json!([])));
    };
    let Some(marker_start) = description.find(MARKER) else {
        return Ok((Some(description), json!([])));
    };
    let fields_start = description[marker_start..]
        .find('\n')
        .map(|offset| marker_start + offset + 1)
        .ok_or(AppError::Internal)?;
    let fields = serde_json::from_str(&description[fields_start..]).map_err(|err| {
        error!(
            target: "nit_v1::indices",
            event = "index_payload_fields_parse_error",
            error = %err,
            "Could not parse stored index fields"
        );
        AppError::Internal
    })?;
    let human_description = description[..marker_start].trim().to_string();
    Ok((
        (!human_description.is_empty()).then_some(human_description),
        fields,
    ))
}

pub async fn list_for_interest(pool: &PgPool, interest_id: i32) -> Result<Vec<Index>, AppError> {
    let indices = repository::list_for_interest(pool, interest_id)
        .await
        .map_err(|err| {
            error!(
                target: "nit_v1::indices",
                event = "interest_indices_list_db_error",
                interest_id,
                error = %err,
                "Could not list indices for interest"
            );
            AppError::Internal
        })?;
    if indices.is_empty() {
        // An empty list is valid only when the parent interest exists.
        sectors::service::get_interest(pool, interest_id).await?;
    }
    Ok(indices)
}

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

    let index = repository::insert_index(pool, interest_id, name, request.description.as_deref())
        .await
        .map_err(|err| match err.as_database_error() {
            Some(db)
                if db.is_foreign_key_violation()
                    && db.constraint() == Some("fk_indices_interest") =>
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
        })?;

    index.ok_or_else(|| {
        warn!(
            target: "nit_v1::indices",
            event = "index_create_conflict",
            interest_id,
            "An index with that name already exists for this interest"
        );
        AppError::IndexAlreadyExists
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

    #[sqlx::test(migrations = "./migrations")]
    async fn gets_the_requested_index_by_id(pool: PgPool) {
        sqlx::query("INSERT INTO interests (id, name) VALUES (10, 'Energy')")
            .execute(&pool)
            .await
            .expect("seed interest");
        sqlx::query(
            "INSERT INTO indices (id, interest_id, name, description)
             VALUES (1, 10, 'Solar', NULL), (2, 10, 'Wind', 'Wind output')",
        )
        .execute(&pool)
        .await
        .expect("seed indices");

        let index = get_index(&pool, 2).await.expect("get index");
        assert_eq!(index.id, 2);
        assert_eq!(index.interest_id, 10);
        assert_eq!(index.name, "Wind");
        assert_eq!(index.description.as_deref(), Some("Wind output"));

        let solar = get_index(&pool, 1)
            .await
            .expect("get index without optional fields");
        assert!(solar.description.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn returns_not_found_for_unknown_index(pool: PgPool) {
        assert!(matches!(
            get_index(&pool, 999).await,
            Err(AppError::IndexNotFound)
        ));
    }

    #[tokio::test]
    async fn getting_an_index_reports_a_database_failure() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        pool.close().await;
        assert!(matches!(get_index(&pool, 1).await, Err(AppError::Internal)));
    }

    fn request(interest_id: i32, name: &str) -> CreateIndexRequest {
        CreateIndexRequest {
            interest_id,
            name: name.into(),
            description: None,
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn enforces_index_name_uniqueness_within_each_interest(pool: PgPool) {
        sqlx::query("INSERT INTO interests (id, name) VALUES (1, 'Energy'), (2, 'Environment')")
            .execute(&pool)
            .await
            .expect("seed interests");

        let first = create_index(&pool, request(1, "Solar"))
            .await
            .expect("create the first index");
        let duplicate = create_index(&pool, request(1, " Solar ")).await;
        assert!(matches!(duplicate, Err(AppError::IndexAlreadyExists)));

        let other = create_index(&pool, request(2, "Solar"))
            .await
            .expect("allow the same name for a different interest");
        assert_ne!(first.id, other.id);
        let stored = list_for_interest(&pool, 1).await.expect("list indices");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, first.id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn lists_only_indices_for_the_requested_interest_in_id_order(pool: PgPool) {
        sqlx::query("INSERT INTO interests (id, name) VALUES (1, 'Energy'), (2, 'Health')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO indices (id, interest_id, name, description)
             VALUES (30, 1, 'Wind', NULL), (20, 2, 'Beds', NULL),
                    (10, 1, 'Solar', 'Solar output')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let rows = list_for_interest(&pool, 1).await.expect("list indices");
        assert_eq!(rows.iter().map(|i| i.id).collect::<Vec<_>>(), [10, 30]);
        assert_eq!(rows[0].interest_id, 1);
        assert_eq!(rows[0].name, "Solar");
        assert_eq!(rows[0].description.as_deref(), Some("Solar output"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn distinguishes_an_empty_interest_from_a_missing_interest(pool: PgPool) {
        sqlx::query("INSERT INTO interests (id, name) VALUES (1, 'Energy')")
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            list_for_interest(&pool, 1)
                .await
                .expect("list indices")
                .is_empty()
        );
        assert!(matches!(
            list_for_interest(&pool, 2).await,
            Err(AppError::InterestNotFound)
        ));
    }

    #[tokio::test]
    async fn listing_indices_reports_a_database_failure() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        pool.close().await;
        assert!(matches!(
            list_for_interest(&pool, 1).await,
            Err(AppError::Internal)
        ));
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
        let created = create_index(&pool, input).await.expect("create index");
        let stored = sqlx::query_as::<_, Index>("SELECT * FROM indices WHERE id = $1")
            .bind(created.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored.interest_id, interest_id);
        assert_eq!(stored.name, "Solar output");
        assert_eq!(stored.description.as_deref(), Some("Daily generation"));
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
