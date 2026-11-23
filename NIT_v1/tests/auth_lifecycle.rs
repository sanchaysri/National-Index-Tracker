use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        Request, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
};
use nit_v1::{build_router, state::AppState};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test(migrations = "./migrations")]
async fn account_can_log_in_and_out_only_from_the_expected_state(pool: PgPool) {
    let app = build_router(AppState { db: pool.clone() });
    let email = "auth-lifecycle@example.com";
    let password = "test-password-123";
    let credentials = json!({ "email": email, "password": password });

    let registration = post_json(
        &app,
        "/api/v1/auth/register",
        &json!({ "name": "Test User", "email": email, "password": password }),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(registration["data"]["email"], email);
    assert_logged_in(&pool, email, false).await;

    let login = post_json(&app, "/api/v1/auth/login", &credentials, StatusCode::OK).await;
    assert_eq!(login["data"]["message"], "Logged in");
    assert_eq!(login["data"]["role"], "user");
    assert_eq!(login["data"]["redirectTo"], "/dashboard");
    assert_logged_in(&pool, email, true).await;

    let repeated_login = post_json(
        &app,
        "/api/v1/auth/login",
        &credentials,
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(repeated_login["error"]["message"], "Already logged in");
    assert_logged_in(&pool, email, true).await;

    let logout = post_json(&app, "/api/v1/auth/logout", &credentials, StatusCode::OK).await;
    assert_eq!(logout["data"]["message"], "Logged out");
    assert_logged_in(&pool, email, false).await;

    let repeated_logout = post_json(
        &app,
        "/api/v1/auth/logout",
        &credentials,
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(repeated_logout["error"]["message"], "Already logged out");
    assert_logged_in(&pool, email, false).await;

    let next_login = post_json(&app, "/api/v1/auth/login", &credentials, StatusCode::OK).await;
    assert_eq!(next_login["data"]["message"], "Logged in");
    assert_logged_in(&pool, email, true).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn logout_revokes_access_to_protected_routes(pool: PgPool) {
    let app = build_router(AppState { db: pool });
    let email = "session-test@example.com";
    let password = "test-password-123";

    post_json(
        &app,
        "/api/v1/auth/register",
        &json!({ "name": "Session Test", "email": email, "password": password }),
        StatusCode::CREATED,
    )
    .await;

    let login = post_json(
        &app,
        "/api/v1/auth/login",
        &json!({ "email": email, "password": password }),
        StatusCode::OK,
    )
    .await;

    let access_token = login["data"]["accessToken"]
        .as_str()
        .filter(|token| !token.is_empty())
        .expect("login should return a non-empty access token");
    let bearer = format!("Bearer {access_token}");

    let protected_request = || {
        Request::get("/api/v1/interests")
            .header(AUTHORIZATION, bearer.as_str())
            .body(Body::empty())
            .expect("valid protected request")
    };

    let before_logout = app
        .clone()
        .oneshot(protected_request())
        .await
        .expect("router response");
    assert_eq!(before_logout.status(), StatusCode::OK);

    let logout_request = Request::post("/api/v1/auth/logout")
        .header(AUTHORIZATION, bearer.as_str())
        .body(Body::empty())
        .expect("valid logout request");
    let logout = app
        .clone()
        .oneshot(logout_request)
        .await
        .expect("router response");
    assert_eq!(logout.status(), StatusCode::OK);

    let after_logout = app
        .oneshot(protected_request())
        .await
        .expect("router response");
    assert_eq!(after_logout.status(), StatusCode::UNAUTHORIZED);
}

async fn post_json(app: &Router, path: &str, body: &Value, status: StatusCode) -> Value {
    let request = Request::post(path)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("valid test request");
    let response = app.clone().oneshot(request).await.expect("router response");

    assert_eq!(response.status(), status, "POST {path}");
    let body = to_bytes(response.into_body(), 4096)
        .await
        .expect("read response body");
    serde_json::from_slice(&body).expect("JSON response body")
}

async fn assert_logged_in(pool: &PgPool, email: &str, expected: bool) {
    let flag: bool = sqlx::query_scalar("SELECT is_logged_in FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("read the account's is_logged_in flag");

    assert_eq!(flag, expected, "stored login flag for {email}");
}
