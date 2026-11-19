use nit_v1::features::auth::repository;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn unknown_email_should_return_none(pool: PgPool) {
    // ARRANGE
    let email = "unknown@example.com";

    // ACT
    let result = repository::find_user_by_email(&pool, email).await;

    // ASSERT
    assert!(result.is_ok());

    let user = result.unwrap();

    assert!(user.is_none());
}
