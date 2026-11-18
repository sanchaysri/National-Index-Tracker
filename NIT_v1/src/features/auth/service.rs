use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use email_address::{EmailAddress, Options};
use sqlx::PgPool;

use crate::error::AppError;

use super::{
    model::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse},
    repository,
};

pub async fn register_user(
    pool: &PgPool,
    request: RegisterRequest,
) -> Result<RegisterResponse, AppError> {
    // Step 1: Validate name
    let name = request.name.trim();

    if name.is_empty() || name.chars().count() > 100 {
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
        return Err(AppError::BadRequest("Invalid email address".into()));
    }

    // Step 3: Validate password
    if request.password.len() < 12 || request.password.len() > 128 {
        return Err(AppError::BadRequest(
            "Password must be 12 to 128 bytes".into(),
        ));
    }

    // Step 4: Generate Argon2id password hash
    let password = request.password;

    let password_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);

        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| AppError::Internal)?
    .map_err(|_| AppError::Internal)?;

    // Step 5: Insert user into PostgreSQL
    let user = repository::insert_user(pool, name, &email, &password_hash)
        .await
        .map_err(|error| match &error {
            sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("23505") => {
                AppError::Conflict("Email is already registered".into())
            }

            _ => {
                eprintln!("Registration database error: {error}");
                AppError::Internal
            }
        })?;

    // Step 6: Return public user information
    Ok(RegisterResponse::from(user))
}

fn role_destination(is_admin: bool) -> (&'static str, &'static str) {
    if is_admin {
        ("admin", "/admin/dashboard")
    } else {
        ("user", "/dashboard")
    }
}

pub async fn login_user(pool: &PgPool, request: LoginRequest) -> Result<LoginResponse, AppError> {
    // Step 1: Normalize email
    let email = request.email.trim().to_lowercase();

    // Step 2: Retrieve user by email
    let user = repository::find_user_by_email(pool, &email)
        .await
        .map_err(|error| {
            eprintln!("Login database error: {error}");
            AppError::Internal
        })?;

    // Step 3: Return Unauthorized if user does not exist
    let user = user.ok_or(AppError::Unauthorized)?;

    // Step 4: Move owned data into blocking task
    let password = request.password;
    let stored_hash = user.password_hash;

    // Step 5: Verify password using Argon2id
    let password_valid = tokio::task::spawn_blocking(move || {
        let parsed_hash = PasswordHash::new(&stored_hash).map_err(|_| AppError::Internal)?;

        Ok::<bool, AppError>(
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok(),
        )
    })
    .await
    .map_err(|_| AppError::Internal)??;

    if !password_valid {
        return Err(AppError::Unauthorized);
    }

    // Step 6: Determine user role
    let (role, redirect_to) = role_destination(user.is_admin);

    // Step 7: Return login response
    Ok(LoginResponse {
        message: "Logged in".to_string(),
        role: role.to_string(),
        redirect_to: redirect_to.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::role_destination;

    #[test]
    fn admin_should_get_admin_dashboard() {
        // Arrange
        let is_admin = true;

        // Act
        let (role, redirect_to) = role_destination(is_admin);

        // Assert
        assert_eq!(role, "admin");
        assert_eq!(redirect_to, "/admin/dashboard");
    }

    #[test]
    fn normal_user_should_get_user_dashboard() {
        // Arrange
        let is_admin = false;

        // Act
        let (role, redirect_to) = role_destination(is_admin);

        // Assert
        assert_eq!(role, "user");
        assert_eq!(redirect_to, "/dashboard");
    }
}
