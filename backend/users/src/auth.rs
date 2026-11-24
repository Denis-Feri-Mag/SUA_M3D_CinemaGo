use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::http::HeaderMap;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::db;
use crate::errors::ApiError;
use crate::models::UserRow;
use crate::AppState;

// Data stored inside the JWT
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub token_version: i32,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("could not hash password")
        .to_string()
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(parsed_hash) => parsed_hash,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn create_token(user: &UserRow, jwt_secret: &str) -> String {
    // Token is valid for 24 hours
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);
    let claims = Claims {
        sub: user.id.to_string(),
        role: user.role.clone(),
        token_version: user.token_version,
        exp: expires_at.timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("could not create token")
}

// Reads "Authorization: Bearer <token>" and returns the logged in user
pub async fn get_logged_in_user(headers: &HeaderMap, state: &AppState) -> Result<UserRow, ApiError> {
    let header_value = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::unauthorized())?;

    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or(ApiError::unauthorized())?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| ApiError::unauthorized())?;

    let user_id: i32 = token_data
        .claims
        .sub
        .parse()
        .map_err(|_| ApiError::unauthorized())?;

    let user = db::get_user_by_id(&state.pool, user_id)
        .await?
        .ok_or(ApiError::unauthorized())?;

    // After logout the version goes up, so old tokens are rejected
    if user.token_version != token_data.claims.token_version {
        return Err(ApiError::unauthorized());
    }

    Ok(user)
}

// A user can change their own account, an admin can change any account
pub fn check_self_or_admin(logged_in_user: &UserRow, user_id: i32) -> Result<(), ApiError> {
    if logged_in_user.id == user_id || logged_in_user.role == "admin" {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}
