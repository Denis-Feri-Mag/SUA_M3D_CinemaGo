use axum::http::HeaderMap;

use crate::errors::ApiError;
use crate::models::user::CachedUser;
use crate::services::auth_service;
use crate::state::AppState;

// Reads "Authorization: Bearer <token>" and returns the logged in user.
// The token itself is checked in the auth service.
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

    auth_service::find_user_by_token(state, token).await
}
