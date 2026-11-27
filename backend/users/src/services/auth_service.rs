// Business rules for accounts and tokens: register, login, refresh, logout, password.
// Controllers call these functions, these functions call the repositories (db/).

use axum::http::StatusCode;
use validator::Validate;

use crate::auth::{jwt, password, refresh_token};
use crate::cache::user_cache;
use crate::clients::notifications;
use crate::db::{refresh_token_repository, user_repository};
use crate::errors::ApiError;
use crate::models::requests::{
    ChangePasswordRequest, LoginRequest, RefreshRequest, RegisterRequest,
};
use crate::models::responses::{LoginResponse, RefreshResponse, UserResponse};
use crate::models::user::{CachedUser, ROLE_ADMIN, ROLE_USER};
use crate::services::user_service;
use crate::state::AppState;

const REFRESH_TOKEN_DAYS: i64 = 7;

pub async fn register(
    state: &AppState,
    request: RegisterRequest,
) -> Result<UserResponse, ApiError> {
    request.validate()?;
    let (name, email) = user_service::clean_name_and_email(&request.name, &request.email)?;

    // The first user becomes admin, everyone else is a normal user
    let user_count = user_repository::count_users(&state.pool).await?;
    let role = if user_count == 0 {
        ROLE_ADMIN
    } else {
        ROLE_USER
    };

    let password_hash = password::hash_password(&request.password);
    let user =
        user_repository::insert_user(&state.pool, &name, &email, &password_hash, role).await?;

    notifications::send_welcome(state, user.id, &user.name).await;
    Ok(user.to_response())
}

pub async fn login(state: &AppState, request: LoginRequest) -> Result<LoginResponse, ApiError> {
    let email = request.email.trim().to_lowercase();
    let wrong_login = ApiError::new(StatusCode::UNAUTHORIZED, "wrong email or password");

    let user = match user_repository::get_user_by_email(&state.pool, &email).await? {
        Some(user) => user,
        None => return Err(wrong_login),
    };
    if !password::verify_password(&request.password, &user.password_hash) {
        return Err(wrong_login);
    }

    let token = jwt::create_token(&user, &state.jwt_secret, state.access_token_minutes);
    let new_refresh_token = create_refresh_token(state, user.id).await?;
    Ok(LoginResponse {
        token,
        refresh_token: new_refresh_token,
        user: user.to_response(),
    })
}

pub async fn refresh(
    state: &AppState,
    request: RefreshRequest,
) -> Result<RefreshResponse, ApiError> {
    let invalid_token = ApiError::new(StatusCode::UNAUTHORIZED, "invalid refresh token");

    // The old token is deleted here, so it cannot be used again
    let token_hash = refresh_token::hash_refresh_token(&request.refresh_token);
    let saved_token =
        match refresh_token_repository::take_refresh_token(&state.pool, &token_hash).await? {
            Some(saved_token) => saved_token,
            None => return Err(invalid_token),
        };
    if saved_token.expires_at < chrono::Utc::now() {
        return Err(invalid_token);
    }

    let user = match user_repository::get_user_by_id(&state.pool, saved_token.user_id).await? {
        Some(user) => user,
        None => return Err(invalid_token),
    };

    let token = jwt::create_token(&user, &state.jwt_secret, state.access_token_minutes);
    let new_refresh_token = create_refresh_token(state, user.id).await?;
    Ok(RefreshResponse {
        token,
        refresh_token: new_refresh_token,
    })
}

// Old access tokens stop working because the token version goes up
pub async fn logout(
    state: &AppState,
    logged_in_user: &CachedUser,
    user_id: i32,
) -> Result<(), ApiError> {
    user_service::check_self_or_admin(logged_in_user, user_id)?;

    let was_updated = user_repository::increase_token_version(&state.pool, user_id).await?;
    // The cached user still has the old token version
    user_cache::remove_user(state, user_id).await;
    if !was_updated {
        return Err(ApiError::not_found());
    }
    refresh_token_repository::delete_refresh_tokens_for_user(&state.pool, user_id).await?;
    Ok(())
}

pub async fn change_password(
    state: &AppState,
    logged_in_user: &CachedUser,
    request: ChangePasswordRequest,
) -> Result<(), ApiError> {
    // The cached user has no password hash, so read it from the database
    let user = user_repository::get_user_by_id(&state.pool, logged_in_user.id)
        .await?
        .ok_or(ApiError::unauthorized())?;
    if !password::verify_password(&request.old_password, &user.password_hash) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "old password is wrong",
        ));
    }
    request.validate()?;

    let password_hash = password::hash_password(&request.new_password);
    user_repository::update_password(&state.pool, user.id, &password_hash).await?;
    refresh_token_repository::delete_refresh_tokens_for_user(&state.pool, user.id).await?;
    user_cache::remove_user(state, user.id).await;
    Ok(())
}

// Checks the access token and returns its user (used for every protected request)
pub async fn find_user_by_token(state: &AppState, token: &str) -> Result<CachedUser, ApiError> {
    let claims = jwt::read_token(token, &state.jwt_secret).ok_or(ApiError::unauthorized())?;
    let user_id: i32 = claims.sub.parse().map_err(|_| ApiError::unauthorized())?;

    // Runs on every protected request, so the user comes from the cache when possible
    let user = user_cache::find_user(state, user_id)
        .await?
        .ok_or(ApiError::unauthorized())?;

    // After logout or a password change the version goes up, so old tokens are rejected
    if user.token_version != claims.token_version {
        return Err(ApiError::unauthorized());
    }
    Ok(user)
}

// Creates a refresh token valid for 7 days and saves only its hash
async fn create_refresh_token(state: &AppState, user_id: i32) -> Result<String, ApiError> {
    let new_refresh_token = refresh_token::generate_refresh_token();
    let token_hash = refresh_token::hash_refresh_token(&new_refresh_token);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(REFRESH_TOKEN_DAYS);
    refresh_token_repository::insert_refresh_token(&state.pool, user_id, &token_hash, expires_at)
        .await?;
    Ok(new_refresh_token)
}
