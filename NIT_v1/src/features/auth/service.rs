use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use sqlx::PgPool;

use crate::error::AppError;

use super::{
    model::{RegisterRequest, RegisterResponse},
    repository,
};

pub async fn register_user(
    pool: &PgPool,
    request: RegisterRequest,
) -> Result<RegisterResponse, AppError> {
    // 1. Validate name
    let name = request.name.trim();

    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::BadRequest(
            "Name must contain 1 to 100 characters".into(),
        ));
    }

    // 2. Normalize and validate email
    let email = request.email.trim().to_lowercase();

    if email.len() > 255 || !email.contains('@') || email.contains(char::is_whitespace) {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }

    // 3. Validate password
    if request.password.len() < 12 || request.password.len() > 128 {
        return Err(AppError::BadRequest(
            "Password must be 12 to 128 bytes".into(),
        ));
    }

    // 4. Hash password on a blocking worker thread
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

    // 5. Save user through repository
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

    // 6. Return only public fields
    Ok(RegisterResponse::from(user))
}
