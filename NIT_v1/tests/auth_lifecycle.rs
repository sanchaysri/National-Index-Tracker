use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        Request, StatusCode,
        header::{
            AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, COOKIE, SET_COOKIE, WWW_AUTHENTICATE,
        },
    },
    response::Response,
};
use nit_v1::{build_router, state::AppState};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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

    let (first, second) = tokio::join!(
        post(&app, "/api/v1/auth/login", &credentials),
        post(&app, "/api/v1/auth/login", &credentials),
    );
    let (login, repeated_login) = if first.status() == StatusCode::OK {
        (first, second)
    } else {
        (second, first)
    };
    assert_eq!(login.status(), StatusCode::OK);
    assert_eq!(login.headers()[CACHE_CONTROL], "no-store");
    let cookie = login.headers()[SET_COOKIE]
        .to_str()
        .expect("refresh cookie");
    let mut cookie_parts = cookie.split("; ");
    let refresh_token = cookie_parts
        .next()
        .and_then(|part| part.strip_prefix("refresh_token="))
        .expect("refresh token")
        .to_owned();
    let attributes: Vec<_> = cookie_parts.collect();
    for expected in [
        "HttpOnly",
        "Secure",
        "SameSite=Lax",
        "Path=/api/v1/auth",
        "Max-Age=604800",
    ] {
        assert!(attributes.contains(&expected));
    }

    let login = json_body(login).await;
    assert_eq!(login["data"]["message"], "Logged in");
    assert_eq!(login["data"]["role"], "user");
    assert_eq!(login["data"]["redirectTo"], "/dashboard");
    let access_token = login["data"]["accessToken"].as_str().expect("access token");
    assert_eq!(hex::decode(access_token).unwrap().len(), 32);
    assert_eq!(hex::decode(&refresh_token).unwrap().len(), 32);
    assert_ne!(access_token, refresh_token);
    assert!(login["data"].get("refreshToken").is_none());
    assert_logged_in(&pool, email, true).await;

    let (access_hash, refresh_hash, expected_expiries): (Vec<u8>, Vec<u8>, bool) = sqlx::query_as(
        "SELECT access_token_hash, refresh_token_hash,
                access_expires_at - created_at = INTERVAL '15 minutes'
                AND refresh_expires_at - created_at = INTERVAL '7 days'
         FROM user_sessions WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(registration["data"]["id"].as_i64().unwrap())
    .fetch_one(&pool)
    .await
    .expect("active session");
    assert_eq!(
        access_hash,
        Sha256::digest(access_token.as_bytes()).to_vec()
    );
    assert_eq!(
        refresh_hash,
        Sha256::digest(refresh_token.as_bytes()).to_vec()
    );
    assert!(expected_expiries);

    assert_eq!(repeated_login.status(), StatusCode::CONFLICT);
    assert!(!repeated_login.headers().contains_key(SET_COOKIE));
    let repeated_login = json_body(repeated_login).await;
    assert_eq!(repeated_login["error"]["message"], "Already logged in");
    assert_logged_in(&pool, email, true).await;

    let bearer = format!("Bearer {access_token}");
    let logout = logout_request(&app, Some(&bearer)).await;
    assert_eq!(logout.status(), StatusCode::OK);
    let logout = json_body(logout).await;
    assert_eq!(logout["data"]["message"], "Logged out");
    assert_logged_in(&pool, email, false).await;

    let repeated_logout = logout_request(&app, Some(&bearer)).await;
    assert_eq!(repeated_logout.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        json_body(repeated_logout).await["error"]["code"],
        "INVALID_ACCESS_TOKEN"
    );
    assert_logged_in(&pool, email, false).await;

    let next_login = post_json(&app, "/api/v1/auth/login", &credentials, StatusCode::OK).await;
    assert_eq!(next_login["data"]["message"], "Logged in");
    assert_logged_in(&pool, email, true).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn protected_routes_require_an_unexpired_unrevoked_access_token(pool: PgPool) {
    let app = build_router(AppState { db: pool.clone() });
    let email = "protected-routes@example.com";
    let password = "test-password-123";
    let credentials = json!({ "email": email, "password": password });

    for (method, path) in [
        ("GET", "/api/v1/interests"),
        ("POST", "/api/v1/interests"),
        ("GET", "/api/v1/interests/1"),
        ("DELETE", "/api/v1/interests/1"),
        ("GET", "/api/v1/interests/1/indices"),
        ("POST", "/api/v1/indices"),
        ("DELETE", "/api/v1/indices/1"),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {path}"
        );
        assert_eq!(response.headers()[WWW_AUTHENTICATE], "Bearer");
    }
    let health = Request::get("/api/v1/health").body(Body::empty()).unwrap();
    assert_eq!(
        app.clone().oneshot(health).await.unwrap().status(),
        StatusCode::OK
    );

    post_json(
        &app,
        "/api/v1/auth/register",
        &json!({ "name": "Protected Routes Test", "email": email, "password": password }),
        StatusCode::CREATED,
    )
    .await;
    let login = post_json(&app, "/api/v1/auth/login", &credentials, StatusCode::OK).await;
    let token = login["data"]["accessToken"].as_str().unwrap();
    let bearer = format!("bearer {token}");
    assert_eq!(interests_status(&app, &bearer).await, StatusCode::OK);
    assert_eq!(
        interests_status(&app, "Bearer malformed").await,
        StatusCode::UNAUTHORIZED
    );
    let unknown_bearer = format!("Bearer {}", "0".repeat(64));
    assert_eq!(
        interests_status(&app, &unknown_bearer).await,
        StatusCode::UNAUTHORIZED
    );

    assert_eq!(
        logout_request(&app, Some(&bearer)).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        interests_status(&app, &bearer).await,
        StatusCode::UNAUTHORIZED
    );

    let login = post_json(&app, "/api/v1/auth/login", &credentials, StatusCode::OK).await;
    let token = login["data"]["accessToken"].as_str().unwrap();
    let token_hash = Sha256::digest(token.as_bytes());
    sqlx::query(
        "UPDATE user_sessions
         SET created_at = statement_timestamp() - INTERVAL '2 minutes',
             access_expires_at = statement_timestamp() - INTERVAL '1 minute'
         WHERE access_token_hash = $1",
    )
    .bind(&token_hash[..])
    .execute(&pool)
    .await
    .expect("expire access token");
    assert_eq!(
        interests_status(&app, &format!("Bearer {token}")).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        logout_request(&app, Some(&format!("Bearer {token}")))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn interest_and_index_writes_require_an_admin(pool: PgPool) {
    let app = build_router(AppState { db: pool.clone() });
    let email = "admin-access@example.com";
    let password = "test-password-123";
    let credentials = json!({ "email": email, "password": password });
    post_json(
        &app,
        "/api/v1/auth/register",
        &json!({ "name": "Admin Access Test", "email": email, "password": password }),
        StatusCode::CREATED,
    )
    .await;
    let login = post_json(&app, "/api/v1/auth/login", &credentials, StatusCode::OK).await;
    let bearer = format!("Bearer {}", login["data"]["accessToken"].as_str().unwrap());
    let unknown_bearer = format!("Bearer {}", "0".repeat(64));
    let interest_body = json!({ "name": "Energy" });

    for (method, path, body) in [
        ("POST", "/api/v1/interests", Some(interest_body.clone())),
        ("DELETE", "/api/v1/interests/1", None),
        (
            "POST",
            "/api/v1/indices",
            Some(json!({ "interest_id": 1, "name": "Production" })),
        ),
        ("DELETE", "/api/v1/indices/1", None),
    ] {
        let invalid = catalog_request(&app, method, path, &unknown_bearer, body.as_ref()).await;
        assert_eq!(
            invalid.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {path}"
        );
        let forbidden = catalog_request(&app, method, path, &bearer, body.as_ref()).await;
        assert_eq!(forbidden.status(), StatusCode::FORBIDDEN, "{method} {path}");
        assert_eq!(
            json_body(forbidden).await["error"]["code"],
            "ADMIN_REQUIRED"
        );
    }

    // The access token should use the latest role in PostgreSQL on each request.
    let set_admin = |is_admin: bool| {
        sqlx::query("UPDATE users SET is_admin = $1 WHERE email = $2")
            .bind(is_admin)
            .bind(email)
            .execute(&pool)
    };
    set_admin(true).await.unwrap();
    let interest = catalog_request(
        &app,
        "POST",
        "/api/v1/interests",
        &bearer,
        Some(&interest_body),
    )
    .await;
    assert_eq!(interest.status(), StatusCode::CREATED);
    let interest = json_body(interest).await;
    let interest_id = interest["id"].as_i64().unwrap();
    let index_body = json!({ "interest_id": interest_id, "name": "Production" });
    let index = catalog_request(&app, "POST", "/api/v1/indices", &bearer, Some(&index_body)).await;
    assert_eq!(index.status(), StatusCode::CREATED);
    let index = json_body(index).await;
    let index_id = index["id"].as_i64().unwrap();

    let interest_path = format!("/api/v1/interests/{interest_id}");
    let interest_indices_path = format!("{interest_path}/indices");
    let index_path = format!("/api/v1/indices/{index_id}");
    set_admin(false).await.unwrap();
    for path in [
        "/api/v1/interests",
        &interest_path,
        &interest_indices_path,
        &index_path,
    ] {
        let response = catalog_request(&app, "GET", path, &bearer, None).await;
        assert_eq!(response.status(), StatusCode::OK, "GET {path}");
    }
    assert_eq!(
        catalog_request(&app, "DELETE", &index_path, &bearer, None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );

    set_admin(true).await.unwrap();
    for path in [&index_path, &interest_path] {
        let response = catalog_request(&app, "DELETE", path, &bearer, None).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT, "DELETE {path}");
    }
}

async fn catalog_request(
    app: &Router,
    method: &str,
    path: &str,
    bearer: &str,
    body: Option<&Value>,
) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(AUTHORIZATION, bearer);
    let body = if let Some(body) = body {
        request = request.header(CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn interests_status(app: &Router, authorization: &str) -> StatusCode {
    let request = Request::get("/api/v1/interests")
        .header(AUTHORIZATION, authorization)
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(request).await.unwrap().status()
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_rotates_both_tokens_in_the_same_session(pool: PgPool) {
    let app = build_router(AppState { db: pool.clone() });
    let credentials = json!({
        "name": "Refresh Test", "email": "refresh@example.com", "password": "test-password-123"
    });
    post_json(
        &app,
        "/api/v1/auth/register",
        &credentials,
        StatusCode::CREATED,
    )
    .await;
    let login = post(&app, "/api/v1/auth/login", &credentials).await;
    assert_eq!(login.status(), StatusCode::OK);
    let old_cookie = refresh_cookie(&login).to_owned();
    let login = json_body(login).await;
    let old_bearer = format!("Bearer {}", login["data"]["accessToken"].as_str().unwrap());
    let session_id: i64 = sqlx::query_scalar(
        "UPDATE user_sessions
         SET created_at = statement_timestamp() - INTERVAL '2 minutes',
             access_expires_at = statement_timestamp() - INTERVAL '1 minute',
             refresh_expires_at = statement_timestamp() + INTERVAL '1 minute'
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        interests_status(&app, &old_bearer).await,
        StatusCode::UNAUTHORIZED
    );

    let (first, second) = tokio::join!(
        refresh_request(&app, Some(&old_cookie)),
        refresh_request(&app, Some(&old_cookie)),
    );
    let (success, replay) = if first.status() == StatusCode::OK {
        (first, second)
    } else {
        (second, first)
    };
    assert_eq!(success.status(), StatusCode::OK);
    assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);
    assert!(!replay.headers().contains_key(SET_COOKIE));
    assert_eq!(success.headers()[CACHE_CONTROL], "no-store");
    let new_cookie = refresh_cookie(&success).to_owned();
    assert_ne!(old_cookie, new_cookie);
    let refreshed = json_body(success).await;
    let access_token = refreshed["data"]["accessToken"].as_str().unwrap();
    assert_eq!(
        interests_status(&app, &format!("Bearer {access_token}")).await,
        StatusCode::OK
    );
    assert_eq!(
        interests_status(&app, &old_bearer).await,
        StatusCode::UNAUTHORIZED
    );

    let rows: Vec<(i64, Vec<u8>, Vec<u8>, bool)> = sqlx::query_as(
        "SELECT id, access_token_hash, refresh_token_hash,
                access_expires_at > statement_timestamp()
                AND refresh_expires_at > statement_timestamp() + INTERVAL '6 days'
         FROM user_sessions",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let refresh_token = new_cookie.strip_prefix("refresh_token=").unwrap();
    assert_eq!(
        rows,
        vec![(
            session_id,
            Sha256::digest(access_token.as_bytes()).to_vec(),
            Sha256::digest(refresh_token.as_bytes()).to_vec(),
            true,
        )]
    );

    assert_eq!(
        refresh_request(&app, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    sqlx::query(
        "UPDATE user_sessions
         SET access_expires_at = statement_timestamp() - INTERVAL '1 minute',
             refresh_expires_at = statement_timestamp() - INTERVAL '1 minute'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        refresh_request(&app, Some(&new_cookie)).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

fn refresh_cookie(response: &Response) -> &str {
    response.headers()[SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
}

async fn refresh_request(app: &Router, cookie: Option<&str>) -> Response {
    let mut request = Request::post("/api/v1/auth/refresh");
    if let Some(cookie) = cookie {
        request = request.header(COOKIE, cookie);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
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

    let login = post(
        &app,
        "/api/v1/auth/login",
        &json!({ "email": email, "password": password }),
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = refresh_cookie(&login).to_owned();
    let login = json_body(login).await;

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

    assert_eq!(
        logout_request(&app, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let logout = logout_request(&app, Some(&bearer)).await;
    assert_eq!(logout.status(), StatusCode::OK);
    let cleared_cookie = logout.headers()[SET_COOKIE].to_str().unwrap();
    assert_eq!(
        cleared_cookie,
        "refresh_token=; HttpOnly; Secure; SameSite=Lax; Path=/api/v1/auth; Max-Age=0"
    );
    assert_eq!(
        refresh_request(&app, Some(&cookie)).await.status(),
        StatusCode::UNAUTHORIZED
    );

    let after_logout = app
        .oneshot(protected_request())
        .await
        .expect("router response");
    assert_eq!(after_logout.status(), StatusCode::UNAUTHORIZED);
}

async fn logout_request(app: &Router, authorization: Option<&str>) -> Response {
    let mut request = Request::post("/api/v1/auth/logout");
    if let Some(authorization) = authorization {
        request = request.header(AUTHORIZATION, authorization);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn post_json(app: &Router, path: &str, body: &Value, status: StatusCode) -> Value {
    let response = post(app, path, body).await;
    assert_eq!(response.status(), status, "POST {path}");
    json_body(response).await
}

async fn post(app: &Router, path: &str, body: &Value) -> Response {
    let request = Request::post(path)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("valid test request");
    app.clone().oneshot(request).await.expect("router response")
}

async fn json_body(response: Response) -> Value {
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
