use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use email_address::{EmailAddress, Options};
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use crate::error::AppError;

use super::{
    model::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse},
    repository,
};

// ========================================
// REGISTRATION ERROR HELPERS
// ========================================

// Log password-hashing failures without exposing secrets.
fn hashing_error(reason: &'static str) -> AppError {
    error!(
        target: "nit_v1::auth::registration",
        event = "registration_hashing_error",
        reason = reason,
        "Registration password hashing failed"
    );

    AppError::Internal
}

// Handle registration database errors.
fn registration_db_error(error: sqlx::Error) -> AppError {
    match &error {
        sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("23505") => {
            warn!(
                target: "nit_v1::auth::registration",
                event = "registration_conflict",
                reason = "duplicate_email",
                "Registration rejected because email already exists"
            );

            AppError::Conflict("Email is already registered".into())
        }

        _ => {
            error!(
                target: "nit_v1::auth::registration",
                event = "registration_db_error",
                error_kind = "database_operation_failed",
                "Unexpected registration database failure"
            );

            AppError::Internal
        }
    }
}

// ========================================
// REGISTRATION SERVICE
// ========================================

pub async fn register_user(
    pool: &PgPool,
    request: RegisterRequest,
) -> Result<RegisterResponse, AppError> {
    // DEBUG: Registration validation started
    debug!(
        target: "nit_v1::auth::registration",
        event = "registration_validation",
        "Validating registration details"
    );

    // Step 1: Validate name
    let name = request.name.trim();

    if name.is_empty() || name.chars().count() > 100 {
        warn!(
            target: "nit_v1::auth::registration",
            event = "registration_validation_failed",
            field = "name",
            "Registration rejected due to invalid name"
        );

        return Err(AppError::BadRequest(
            "Name must contain 1 to 100 characters".into(),
        ));
    }

    // Step 2: Normalize and validate email
    let email = request.email.trim().to_lowercase();

    let email_options = Options {
        minimum_sub_domains: 2,
        allow_domain_literal: false,
        allow_display_text: false,
        allow_long_local_parts: false,
    };

    if email.len() > 254
        || email.contains(char::is_whitespace)
        || EmailAddress::parse_with_options(&email, email_options).is_err()
    {
        warn!(
            target: "nit_v1::auth::registration",
            event = "registration_validation_failed",
            field = "email",
            "Registration rejected due to invalid email"
        );

        return Err(AppError::BadRequest("Invalid email address".into()));
    }

    // Step 3: Validate password
    if request.password.len() < 12 || request.password.len() > 128 {
        warn!(
            target: "nit_v1::auth::registration",
            event = "registration_validation_failed",
            field = "password",
            "Registration rejected due to invalid password"
        );

        return Err(AppError::BadRequest(
            "Password must be 12 to 128 bytes".into(),
        ));
    }

    // Step 4: Hash password using Argon2
    let password = request.password;

    let password_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);

        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| hashing_error("blocking_task_failed"))?
    .map_err(|_| hashing_error("argon2_failed"))?;

    // DEBUG: Password hashing completed
    debug!(
        target: "nit_v1::auth::registration",
        event = "registration_password_hashed",
        "Password hashing completed successfully"
    );

    // Step 5: Insert user into PostgreSQL
    debug!(
        target: "nit_v1::auth::registration",
        event = "registration_db_insert",
        "Inserting registered user into PostgreSQL"
    );

    let user = repository::insert_user(pool, name, &email, &password_hash)
        .await
        .map_err(registration_db_error)?;

    // Step 6: Return public user information
    Ok(RegisterResponse::from(user))
}

// ========================================
// LOGIN ROLE HELPER
// ========================================

fn role_destination(is_admin: bool) -> (&'static str, &'static str) {
    if is_admin {
        ("admin", "/admin/dashboard")
    } else {
        ("user", "/dashboard")
    }
}

// ========================================
// LOGIN SERVICE
// ========================================

pub async fn login_user(pool: &PgPool, request: LoginRequest) -> Result<LoginResponse, AppError> {
    // DEBUG 1: Start user lookup
    debug!(
        target: "nit_v1::auth::login",
        event = "login_user_lookup",
        "Looking up user for login"
    );

    // Step 1: Normalize email
    let email = request.email.trim().to_lowercase();

    // Step 2: Find user in PostgreSQL
    let user = repository::find_user_by_email(pool, &email)
        .await
        .map_err(|_| {
            error!(
                target: "nit_v1::auth::login",
                event = "login_db_error",
                "Unexpected database error during login"
            );

            AppError::Internal
        })?;

    // Step 3: Check whether user exists
    let Some(user) = user else {
        warn!(
            target: "nit_v1::auth::login",
            event = "login_invalid_credentials",
            "Login rejected due to invalid credentials"
        );

        return Err(AppError::Unauthorized);
    };

    // DEBUG 2: Start password verification
    debug!(
        target: "nit_v1::auth::login",
        event = "login_password_verification",
        "Verifying login password"
    );

    // Step 4: Move owned data into blocking task
    let password = request.password;
    let stored_hash = user.password_hash;

    // Step 5: Verify password using Argon2
    let password_valid = tokio::task::spawn_blocking(move || {
        let parsed_hash = PasswordHash::new(&stored_hash).map_err(|_| {
            error!(
                target: "nit_v1::auth::login",
                event = "login_hash_error",
                "Stored password hash could not be parsed"
            );

            AppError::Internal
        })?;

        Ok::<bool, AppError>(
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok(),
        )
    })
    .await
    .map_err(|_| {
        error!(
            target: "nit_v1::auth::login",
            event = "login_verification_task_error",
            "Login password verification task failed"
        );

        AppError::Internal
    })??;

    // Step 6: Check password validity
    if !password_valid {
        warn!(
            target: "nit_v1::auth::login",
            event = "login_invalid_credentials",
            "Login rejected due to invalid credentials"
        );

        return Err(AppError::Unauthorized);
    }

    // Step 7: Determine user role
    let (role, redirect_to) = role_destination(user.is_admin);

    info!(
        target: "nit_v1::auth::login",
        event = "login_success",
        user_id = user.id,
        name = %user.name,
        email = %user.email,
        role,
        "User logged in successfully"
    );

    // Step 8: Return login response
    Ok(LoginResponse {
        message: "Logged in".to_string(),
        role: role.to_string(),
        redirect_to: redirect_to.to_string(),
    })
}

// ========================================
// UNIT TESTS
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    use sqlx::postgres::PgPoolOptions;

    use std::{
        io::{self, Write},
        sync::{Arc, Mutex},
        time::Duration,
    };

    // Capture tracing output in memory for assertions.
    #[derive(Clone)]
    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for TestWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn captured_text(output: &Arc<Mutex<Vec<u8>>>) -> String {
        String::from_utf8(output.lock().unwrap().clone()).expect("logging output should be UTF-8")
    }

    // TEST 1: Admin dashboard
    #[test]
    fn test_admin_dashboard() {
        let is_admin = true;
        let (role, redirect_to) = role_destination(is_admin);

        assert_eq!(role, "admin");
        assert_eq!(redirect_to, "/admin/dashboard");
    }

    // TEST 2: User dashboard
    #[test]
    fn test_user_dashboard() {
        let is_admin = false;
        let (role, redirect_to) = role_destination(is_admin);

        assert_eq!(role, "user");
        assert_eq!(redirect_to, "/dashboard");
    }

    // TEST 3: Registration hashing error
    #[test]
    fn test_registration_hashing_error() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = TestWriter(output.clone());

        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_ansi(false)
            .with_max_level(tracing::Level::ERROR)
            .with_writer(move || writer.clone())
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            let first = hashing_error("argon2_failed");
            let second = hashing_error("blocking_task_failed");

            assert!(matches!(first, AppError::Internal));
            assert!(matches!(second, AppError::Internal));
        });

        let logs = captured_text(&output);

        assert!(logs.contains("registration_hashing_error"));
        assert!(logs.contains("argon2_failed"));
        assert!(logs.contains("blocking_task_failed"));
    }

    // TEST 4: Registration database error
    #[tokio::test(flavor = "current_thread")]
    async fn test_registration_database_error() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = TestWriter(output.clone());

        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_ansi(false)
            .with_max_level(tracing::Level::ERROR)
            .with_writer(move || writer.clone())
            .finish();

        let _guard = tracing::subscriber::set_default(subscriber);

        let pool = PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(200))
            .connect_lazy("postgres://test:test@127.0.0.1:1/test")
            .expect("valid test connection string");

        let request = RegisterRequest {
            name: "Error Test User".to_string(),
            email: "db-error-test@example.com".to_string(),
            password: "SecurePassword123!".to_string(),
        };

        let result = register_user(&pool, request).await;

        assert!(matches!(result, Err(AppError::Internal)));

        let logs = captured_text(&output);

        assert!(
            logs.contains("registration_db_error"),
            "Expected registration_db_error log, got: {logs}"
        );
    }
}
