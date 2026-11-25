use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use validator::Validate;

use crate::auth::current_user;
use crate::cache::user_cache;
use crate::db::user_queries;
use crate::errors::ApiError;
use crate::extractors::{Json, Path};
use crate::models::requests::{PageQuery, UpdateRoleRequest, UpdateUserRequest};
use crate::models::responses::{ErrorResponse, UserPage, UserResponse};
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/users",
    tag = "users",
    security(("bearerAuth" = [])),
    params(PageQuery),
    responses(
        (status = 200, description = "One page of users (admin only)", body = UserPage),
        (status = 400, description = "Bad page or pageSize", body = ErrorResponse),
        (status = 401, description = "Missing or invalid token", body = ErrorResponse),
        (status = 403, description = "Not an admin", body = ErrorResponse)
    )
)]
pub async fn get_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Result<Json<UserPage>, ApiError> {
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;
    if logged_in_user.role != "admin" {
        return Err(ApiError::forbidden());
    }

    let Query(page_query) = query?;
    let page = page_query.page.unwrap_or(1);
    let page_size = page_query.page_size.unwrap_or(20);
    if page < 1 || !(1..=100).contains(&page_size) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "page must be at least 1 and pageSize 1 to 100",
        ));
    }

    let offset = (page - 1) * page_size;
    let users = user_queries::get_users_page(&state.pool, page_size, offset).await?;
    let total = user_queries::count_users(&state.pool).await?;
    let mut user_list = Vec::new();
    for user in users {
        user_list.push(user.to_response());
    }
    Ok(Json(UserPage {
        items: user_list,
        page,
        page_size,
        total,
    }))
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
    let user = user_cache::find_user(&state, user_id)
        .await?
        .ok_or(ApiError::not_found())?;
    Ok(Json(user.to_response()))
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
        (status = 400, description = "Invalid name or email", body = ErrorResponse),
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
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;
    current_user::check_self_or_admin(&logged_in_user, user_id)?;

    request.validate()?;
    let name = request.name.trim();
    let email = request.email.trim().to_lowercase();
    if name.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "name: is required"));
    }

    let user = user_queries::update_user(&state.pool, user_id, name, &email)
        .await?
        .ok_or(ApiError::not_found())?;
    user_cache::remove_user(&state, user_id).await;
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
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;
    if logged_in_user.role != "admin" {
        return Err(ApiError::forbidden());
    }

    if request.role != "user" && request.role != "admin" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "role must be user or admin",
        ));
    }

    let user = user_queries::update_role(&state.pool, user_id, &request.role)
        .await?
        .ok_or(ApiError::not_found())?;
    user_cache::remove_user(&state, user_id).await;
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
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;
    current_user::check_self_or_admin(&logged_in_user, user_id)?;

    let was_deleted = user_queries::delete_user(&state.pool, user_id).await?;
    user_cache::remove_user(&state, user_id).await;
    if !was_deleted {
        return Err(ApiError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}
