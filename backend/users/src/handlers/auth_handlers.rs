use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use validator::Validate;

use crate::auth::{current_user, jwt, password, refresh_token};
use crate::cache::user_cache;
use crate::clients::notifications;
use crate::db::{refresh_token_queries, user_queries};
use crate::errors::ApiError;
use crate::extractors::{Json, Path};
use crate::models::requests::{
    ChangePasswordRequest, LoginRequest, RefreshRequest, RegisterRequest,
};
use crate::models::responses::{ErrorResponse, LoginResponse, RefreshResponse, UserResponse};
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/users/register",
    tag = "users",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 400, description = "Invalid name, email or password", body = ErrorResponse),
        (status = 409, description = "Email already exists", body = ErrorResponse),
        (
            status = 429,
            description = "Too many requests, wait Retry-After seconds",
            body = ErrorResponse,
            headers(("Retry-After" = u64, description = "Seconds to wait"))
        )
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    request.validate()?;
    let name = request.name.trim();
    let email = request.email.trim().to_lowercase();

    // Only spaces passes the length check, so check the trimmed name too
    if name.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "name: is required"));
    }

    // The first user becomes admin, everyone else is a normal user
    let user_count = user_queries::count_users(&state.pool).await?;
    let role = if user_count == 0 { "admin" } else { "user" };

    let password_hash = password::hash_password(&request.password);
    let user = user_queries::insert_user(&state.pool, name, &email, &password_hash, role).await?;

    notifications::send_welcome(&state, user.id, &user.name).await;

    Ok((StatusCode::CREATED, Json(user.to_response())))
}

#[utoipa::path(
    post,
    path = "/users/login",
    tag = "users",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in, returns an access token and a refresh token", body = LoginResponse),
        (status = 401, description = "Wrong email or password", body = ErrorResponse),
        (
            status = 429,
            description = "Too many requests, wait Retry-After seconds",
            body = ErrorResponse,
            headers(("Retry-After" = u64, description = "Seconds to wait"))
        )
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let email = request.email.trim().to_lowercase();
    let wrong_login = ApiError::new(StatusCode::UNAUTHORIZED, "wrong email or password");

    let user = match user_queries::get_user_by_email(&state.pool, &email).await? {
        Some(user) => user,
        None => return Err(wrong_login),
    };

    if !password::verify_password(&request.password, &user.password_hash) {
        return Err(wrong_login);
    }

    let token = jwt::create_token(&user, &state.jwt_secret, state.access_token_minutes);
    let new_refresh_token = create_refresh_token(&state, user.id).await?;
    Ok(Json(LoginResponse {
        token,
        refresh_token: new_refresh_token,
        user: user.to_response(),
    }))
}

#[utoipa::path(
    post,
    path = "/users/refresh",
    tag = "users",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "New access token and new refresh token, the old refresh token stops working", body = RefreshResponse),
        (status = 401, description = "Refresh token is invalid, used or expired", body = ErrorResponse),
        (
            status = 429,
            description = "Too many requests, wait Retry-After seconds",
            body = ErrorResponse,
            headers(("Retry-After" = u64, description = "Seconds to wait"))
        )
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let invalid_token = ApiError::new(StatusCode::UNAUTHORIZED, "invalid refresh token");

    // The old token is deleted here, so it cannot be used again
    let token_hash = refresh_token::hash_refresh_token(&request.refresh_token);
    let saved_token =
        match refresh_token_queries::take_refresh_token(&state.pool, &token_hash).await? {
            Some(saved_token) => saved_token,
            None => return Err(invalid_token),
        };

    if saved_token.expires_at < chrono::Utc::now() {
        return Err(invalid_token);
    }

    let user = match user_queries::get_user_by_id(&state.pool, saved_token.user_id).await? {
        Some(user) => user,
        None => return Err(invalid_token),
    };

    let token = jwt::create_token(&user, &state.jwt_secret, state.access_token_minutes);
    let new_refresh_token = create_refresh_token(&state, user.id).await?;
    Ok(Json(RefreshResponse {
        token,
        refresh_token: new_refresh_token,
    }))
}

// Creates a refresh token valid for 7 days and saves its hash
async fn create_refresh_token(state: &AppState, user_id: i32) -> Result<String, ApiError> {
    let new_refresh_token = refresh_token::generate_refresh_token();
    let token_hash = refresh_token::hash_refresh_token(&new_refresh_token);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    refresh_token_queries::insert_refresh_token(&state.pool, user_id, &token_hash, expires_at)
        .await?;
    Ok(new_refresh_token)
}

#[utoipa::path(
    delete,
    path = "/users/{id}/sessions",
    tag = "users",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "User id")),
    responses(
        (status = 204, description = "Logged out, all old access and refresh tokens stop working"),
        (status = 401, description = "Missing or invalid token", body = ErrorResponse),
        (status = 403, description = "Not your account", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;
    current_user::check_self_or_admin(&logged_in_user, user_id)?;

    let was_updated = user_queries::increase_token_version(&state.pool, user_id).await?;
    // The cached user still has the old token version
    user_cache::remove_user(&state, user_id).await;
    if !was_updated {
        return Err(ApiError::not_found());
    }
    refresh_token_queries::delete_refresh_tokens_for_user(&state.pool, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/auth/verify",
    tag = "auth",
    security(("bearerAuth" = [])),
    responses(
        (
            status = 200,
            description = "Token is valid, used by Traefik ForwardAuth",
            headers(
                ("X-User-Id" = i32, description = "Id of the logged in user"),
                ("X-User-Role" = String, description = "user or admin")
            )
        ),
        (status = 401, description = "Missing or invalid token", body = ErrorResponse)
    )
)]
pub async fn verify(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<(StatusCode, [(&'static str, String); 2]), ApiError> {
    // Traefik calls this before every protected request and copies these headers to the target service
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;
    let user_headers = [
        ("X-User-Id", logged_in_user.id.to_string()),
        ("X-User-Role", logged_in_user.role.clone()),
    ];
    Ok((StatusCode::OK, user_headers))
}

#[utoipa::path(
    put,
    path = "/users/me/password",
    tag = "users",
    security(("bearerAuth" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 204, description = "Password changed, log in again because old tokens stop working"),
        (status = 400, description = "New password must have 6 to 128 characters", body = ErrorResponse),
        (status = 401, description = "Missing or invalid token, or wrong old password", body = ErrorResponse)
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<StatusCode, ApiError> {
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;

    // The cached user has no password hash, so read it from the database
    let user = user_queries::get_user_by_id(&state.pool, logged_in_user.id)
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
    user_queries::update_password(&state.pool, user.id, &password_hash).await?;
    refresh_token_queries::delete_refresh_tokens_for_user(&state.pool, user.id).await?;
    user_cache::remove_user(&state, user.id).await;
    Ok(StatusCode::NO_CONTENT)
}
