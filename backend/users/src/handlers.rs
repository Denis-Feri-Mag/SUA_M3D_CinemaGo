use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;

use crate::auth;
use crate::db;
use crate::errors::ApiError;
use crate::models::*;
use crate::notifications;
use crate::AppState;

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses((status = 200, description = "Service is running", body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/users",
    tag = "users",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "All users (admin only)", body = Vec<UserResponse>),
        (status = 401, description = "Missing or invalid token", body = ErrorResponse),
        (status = 403, description = "Not an admin", body = ErrorResponse)
    )
)]
pub async fn get_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserResponse>>, ApiError> {
    let logged_in_user = auth::get_logged_in_user(&headers, &state).await?;
    if logged_in_user.role != "admin" {
        return Err(ApiError::forbidden());
    }

    let users = db::get_all_users(&state.pool).await?;
    let mut user_list = Vec::new();
    for user in users {
        user_list.push(user.to_response());
    }
    Ok(Json(user_list))
}

#[utoipa::path(
    get,
    path = "/users/{id}",
    tag = "users",
    params(("id" = i32, Path, description = "User id")),
    responses(
        (status = 200, description = "One user", body = UserResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<i32>,
) -> Result<Json<UserResponse>, ApiError> {
    // No token needed, other services use this to check a user exists
    let user = db::get_user_by_id(&state.pool, user_id)
        .await?
        .ok_or(ApiError::not_found())?;
    Ok(Json(user.to_response()))
}

#[utoipa::path(
    post,
    path = "/users/register",
    tag = "users",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 400, description = "Missing fields or short password", body = ErrorResponse),
        (status = 409, description = "Email already exists", body = ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let name = request.name.trim();
    let email = request.email.trim().to_lowercase();

    if name.is_empty() || email.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "name and email are required"));
    }
    if request.password.len() < 6 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "password must have at least 6 characters"));
    }

    // The first user becomes admin, everyone else is a normal user
    let user_count = db::count_users(&state.pool).await?;
    let role = if user_count == 0 { "admin" } else { "user" };

    let password_hash = auth::hash_password(&request.password);
    let user = db::insert_user(&state.pool, name, &email, &password_hash, role).await?;

    notifications::send_welcome(&state, user.id, &user.name).await;

    Ok((StatusCode::CREATED, Json(user.to_response())))
}

#[utoipa::path(
    post,
    path = "/users/login",
    tag = "users",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in, returns a JWT", body = LoginResponse),
        (status = 401, description = "Wrong email or password", body = ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let email = request.email.trim().to_lowercase();
    let wrong_login = ApiError::new(StatusCode::UNAUTHORIZED, "wrong email or password");

    let user = match db::get_user_by_email(&state.pool, &email).await? {
        Some(user) => user,
        None => return Err(wrong_login),
    };

    if !auth::verify_password(&request.password, &user.password_hash) {
        return Err(wrong_login);
    }

    let token = auth::create_token(&user, &state.jwt_secret);
    Ok(Json(LoginResponse {
        token,
        user: user.to_response(),
    }))
}

#[utoipa::path(
    put,
    path = "/users/{id}",
    tag = "users",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "User id")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = UserResponse),
        (status = 400, description = "Missing fields", body = ErrorResponse),
        (status = 401, description = "Missing or invalid token", body = ErrorResponse),
        (status = 403, description = "Not your account", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 409, description = "Email already exists", body = ErrorResponse)
    )
)]
pub async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i32>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let logged_in_user = auth::get_logged_in_user(&headers, &state).await?;
    auth::check_self_or_admin(&logged_in_user, user_id)?;

    let name = request.name.trim();
    let email = request.email.trim().to_lowercase();
    if name.is_empty() || email.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "name and email are required"));
    }

    let user = db::update_user(&state.pool, user_id, name, &email)
        .await?
        .ok_or(ApiError::not_found())?;
    Ok(Json(user.to_response()))
}

#[utoipa::path(
    put,
    path = "/users/{id}/role",
    tag = "users",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "User id")),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "Role changed", body = UserResponse),
        (status = 400, description = "Role must be user or admin", body = ErrorResponse),
        (status = 401, description = "Missing or invalid token", body = ErrorResponse),
        (status = 403, description = "Not an admin", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn update_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i32>,
    Json(request): Json<UpdateRoleRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let logged_in_user = auth::get_logged_in_user(&headers, &state).await?;
    if logged_in_user.role != "admin" {
        return Err(ApiError::forbidden());
    }

    if request.role != "user" && request.role != "admin" {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "role must be user or admin"));
    }

    let user = db::update_role(&state.pool, user_id, &request.role)
        .await?
        .ok_or(ApiError::not_found())?;
    Ok(Json(user.to_response()))
}

#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "users",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "User id")),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, description = "Missing or invalid token", body = ErrorResponse),
        (status = 403, description = "Not your account", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    let logged_in_user = auth::get_logged_in_user(&headers, &state).await?;
    auth::check_self_or_admin(&logged_in_user, user_id)?;

    let was_deleted = db::delete_user(&state.pool, user_id).await?;
    if !was_deleted {
        return Err(ApiError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/users/{id}/sessions",
    tag = "users",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "User id")),
    responses(
        (status = 204, description = "Logged out, all old tokens stop working"),
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
    let logged_in_user = auth::get_logged_in_user(&headers, &state).await?;
    auth::check_self_or_admin(&logged_in_user, user_id)?;

    let was_updated = db::increase_token_version(&state.pool, user_id).await?;
    if !was_updated {
        return Err(ApiError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}
