use axum::http::HeaderMap;

use crate::auth::jwt;
use crate::cache::user_cache;
use crate::errors::ApiError;
use crate::models::user::CachedUser;
use crate::state::AppState;

// Reads "Authorization: Bearer <token>" and returns the logged in user
pub async fn get_logged_in_user(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<CachedUser, ApiError> {
    let header_value = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::unauthorized())?;

    let token = header_value
        .strip_prefix("Bearer ")
        .ok_or(ApiError::unauthorized())?;

    let claims = jwt::read_token(token, &state.jwt_secret).ok_or(ApiError::unauthorized())?;

    let user_id: i32 = claims.sub.parse().map_err(|_| ApiError::unauthorized())?;

    // Runs on every protected request, so the user comes from the cache when possible
    let user = user_cache::find_user(state, user_id)
        .await?
        .ok_or(ApiError::unauthorized())?;

    // After logout the version goes up, so old tokens are rejected
    if user.token_version != claims.token_version {
        return Err(ApiError::unauthorized());
    }

    Ok(user)
}

// A user can change their own account, an admin can change any account
pub fn check_self_or_admin(logged_in_user: &CachedUser, user_id: i32) -> Result<(), ApiError> {
    if logged_in_user.id == user_id || logged_in_user.role == "admin" {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}
