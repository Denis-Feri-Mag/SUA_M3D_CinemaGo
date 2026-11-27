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
    tag = "admin",
    summary = "List all users",
    description = "Returns one page of all users, sorted by id.\n\n\
**Who can call it:** admins only. Send `Authorization: Bearer <token>` from login.",
    security(("bearerAuth" = [])),
    params(PageQuery),
    responses(
        (status = 200, description = "One page of users.", body = UserPage),
        (status = 400, description = "`page` or `pageSize` is out of range or not a number.", body = ErrorResponse, examples(
            ("Out of range" = (summary = "page below 1 or pageSize not 1 to 100", value = json!({ "error": "page must be at least 1 and pageSize 1 to 100" }))),
            ("Not a number" = (summary = "page is not a number", value = json!({ "error": "Failed to deserialize query string: page: invalid digit found in string" })))
        )),
        (status = 401, description = "The bearer token is missing, expired, invalid, or was logged out (logout, password change and deleting the user make old tokens stop working).", body = ErrorResponse, example = json!({ "error": "missing or invalid token" })),
        (status = 403, description = "You are logged in but not an admin.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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
    summary = "Get one user",
    description = "Returns one user by id. Read from the Valkey cache when possible.\n\n\
**Who can call it:** inside docker every service can call it without a token (`http://users:8080/users/{id}`), for example to check that a user exists. \
Through the gateway (`/api/users/{id}`) any logged in user can call it with a bearer token.",
    params(("id" = i32, Path, description = "User id", example = 7)),
    responses(
        (status = 200, description = "The user.", body = UserResponse, example = json!({ "id": 7, "name": "Test User", "email": "user@example.com", "role": "user", "createdAt": "2026-11-20T10:15:00Z" })),
        (status = 400, description = "The id in the path is not a whole number.", body = ErrorResponse, example = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })),
        (status = 401, description = "Only through the gateway: the bearer token is missing or invalid.", body = ErrorResponse, example = json!({ "error": "missing or invalid token" })),
        (status = 404, description = "No user with this id.", body = ErrorResponse, example = json!({ "error": "user not found" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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
    summary = "Change name and email of a user",
    description = "Replaces the name and email of a user. Both fields are required.\n\n\
**Who can call it:** the user themselves, or an admin for any user. Send `Authorization: Bearer <token>` from login.\n\n\
**Rules:** name 1 to 100 characters (not only spaces), a valid email that no other account uses. The email is saved in lowercase.",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "User id", example = 7)),
    request_body(content = UpdateUserRequest, description = "New name and email.", example = json!({ "name": "Test User", "email": "user@example.com" })),
    responses(
        (status = 200, description = "User changed.", body = UserResponse, example = json!({ "id": 7, "name": "Test User", "email": "user@example.com", "role": "user", "createdAt": "2026-11-20T10:15:00Z" })),
        (status = 400, description = "Bad id, or name or email breaks a rule.", body = ErrorResponse, examples(
            ("Bad email" = (summary = "Email is not valid", value = json!({ "error": "email: must be a valid email" }))),
            ("Bad name" = (summary = "Name is empty, only spaces or too long", value = json!({ "error": "name: must have 1 to 100 characters" }))),
            ("Bad JSON" = (summary = "The body is not valid JSON", value = json!({ "error": "Failed to parse the request body as JSON: key must be a string at line 1 column 2" }))),
            ("Bad id" = (summary = "The id in the path is not a whole number", value = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })))
        )),
        (status = 401, description = "The bearer token is missing, expired, invalid, or was logged out (logout, password change and deleting the user make old tokens stop working).", body = ErrorResponse, example = json!({ "error": "missing or invalid token" })),
        (status = 403, description = "The id is not your own and you are not an admin.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 404, description = "No user with this id.", body = ErrorResponse, example = json!({ "error": "user not found" })),
        (status = 409, description = "Another account already uses this email.", body = ErrorResponse, example = json!({ "error": "email already exists" })),
        (status = 415, description = "The body is not sent as JSON (missing `Content-Type: application/json`).", body = ErrorResponse, example = json!({ "error": "Expected request with `Content-Type: application/json`" })),
        (status = 422, description = "The JSON is valid but a field is missing or has the wrong type.", body = ErrorResponse, example = json!({ "error": "Failed to deserialize the JSON body into the target type: missing field `email` at line 1 column 2" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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
    tag = "admin",
    summary = "Change the role of a user",
    description = "Makes a user an `admin` or a normal `user`. The new role is in tokens issued after this call, old tokens still have the old role until they expire.\n\n\
**Who can call it:** admins only. Send `Authorization: Bearer <token>` from login.\n\n\
**Rules:** the role must be `user` or `admin`. The last admin cannot be made a normal user, there is always at least one admin.",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "User id", example = 7)),
    request_body(content = UpdateRoleRequest, description = "The new role.", example = json!({ "role": "admin" })),
    responses(
        (status = 200, description = "Role changed.", body = UserResponse, example = json!({ "id": 7, "name": "Test User", "email": "user@example.com", "role": "admin", "createdAt": "2026-11-20T10:15:00Z" })),
        (status = 400, description = "Unknown role or bad id.", body = ErrorResponse, examples(
            ("Bad role" = (summary = "Role is not user or admin", value = json!({ "error": "role must be user or admin" }))),
            ("Bad id" = (summary = "The id in the path is not a whole number", value = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })))
        )),
        (status = 401, description = "The bearer token is missing, expired, invalid, or was logged out (logout, password change and deleting the user make old tokens stop working).", body = ErrorResponse, example = json!({ "error": "missing or invalid token" })),
        (status = 403, description = "You are logged in but not an admin.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 404, description = "No user with this id.", body = ErrorResponse, example = json!({ "error": "user not found" })),
        (status = 409, description = "This is the last admin and the new role is user.", body = ErrorResponse, example = json!({ "error": "cannot remove the last admin" })),
        (status = 415, description = "The body is not sent as JSON (missing `Content-Type: application/json`).", body = ErrorResponse, example = json!({ "error": "Expected request with `Content-Type: application/json`" })),
        (status = 422, description = "The JSON is valid but a field is missing or has the wrong type.", body = ErrorResponse, example = json!({ "error": "Failed to deserialize the JSON body into the target type: missing field `role` at line 1 column 2" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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

    if request.role == "user" {
        check_not_last_admin(&state, user_id).await?;
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
    summary = "Delete a user",
    description = "Deletes the account and its refresh tokens. Old access tokens of this user stop working.\n\n\
**Who can call it:** the user themselves, or an admin for any user. Send `Authorization: Bearer <token>` from login.\n\n\
The last admin cannot be deleted.",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "User id", example = 7)),
    responses(
        (status = 204, description = "User deleted. No body."),
        (status = 400, description = "The id in the path is not a whole number.", body = ErrorResponse, example = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })),
        (status = 401, description = "The bearer token is missing, expired, invalid, or was logged out (logout, password change and deleting the user make old tokens stop working).", body = ErrorResponse, example = json!({ "error": "missing or invalid token" })),
        (status = 403, description = "The id is not your own and you are not an admin.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 404, description = "No user with this id.", body = ErrorResponse, example = json!({ "error": "user not found" })),
        (status = 409, description = "This is the last admin.", body = ErrorResponse, example = json!({ "error": "cannot remove the last admin" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;
    current_user::check_self_or_admin(&logged_in_user, user_id)?;

    check_not_last_admin(&state, user_id).await?;

    let was_deleted = user_queries::delete_user(&state.pool, user_id).await?;
    user_cache::remove_user(&state, user_id).await;
    if !was_deleted {
        return Err(ApiError::not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/users/me",
    tag = "me",
    summary = "Get my account",
    description = "Returns the user of the bearer token.\n\n\
**Who can call it:** any logged in user. Send `Authorization: Bearer <token>` from login.",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "The logged in user.", body = UserResponse, example = json!({ "id": 7, "name": "Test User", "email": "user@example.com", "role": "user", "createdAt": "2026-11-20T10:15:00Z" })),
        (status = 401, description = "The bearer token is missing, expired, invalid, or was logged out (logout, password change and deleting the user make old tokens stop working).", body = ErrorResponse, example = json!({ "error": "missing or invalid token" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserResponse>, ApiError> {
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;
    Ok(Json(logged_in_user.to_response()))
}

#[utoipa::path(
    put,
    path = "/users/me",
    tag = "me",
    summary = "Change my name and email",
    description = "Same as PUT /users/{id} with your own id, without needing to know it.\n\n\
**Who can call it:** any logged in user. Send `Authorization: Bearer <token>` from login.\n\n\
**Rules:** name 1 to 100 characters (not only spaces), a valid email that no other account uses. The email is saved in lowercase.",
    security(("bearerAuth" = [])),
    request_body(content = UpdateUserRequest, description = "New name and email.", example = json!({ "name": "Test User", "email": "user@example.com" })),
    responses(
        (status = 200, description = "Account changed.", body = UserResponse, example = json!({ "id": 7, "name": "Test User", "email": "user@example.com", "role": "user", "createdAt": "2026-11-20T10:15:00Z" })),
        (status = 400, description = "Name or email breaks a rule.", body = ErrorResponse, examples(
            ("Bad email" = (summary = "Email is not valid", value = json!({ "error": "email: must be a valid email" }))),
            ("Bad name" = (summary = "Name is empty, only spaces or too long", value = json!({ "error": "name: must have 1 to 100 characters" }))),
            ("Bad JSON" = (summary = "The body is not valid JSON", value = json!({ "error": "Failed to parse the request body as JSON: key must be a string at line 1 column 2" })))
        )),
        (status = 401, description = "The bearer token is missing, expired, invalid, or was logged out (logout, password change and deleting the user make old tokens stop working).", body = ErrorResponse, example = json!({ "error": "missing or invalid token" })),
        (status = 409, description = "Another account already uses this email.", body = ErrorResponse, example = json!({ "error": "email already exists" })),
        (status = 415, description = "The body is not sent as JSON (missing `Content-Type: application/json`).", body = ErrorResponse, example = json!({ "error": "Expected request with `Content-Type: application/json`" })),
        (status = 422, description = "The JSON is valid but a field is missing or has the wrong type.", body = ErrorResponse, example = json!({ "error": "Failed to deserialize the JSON body into the target type: missing field `email` at line 1 column 2" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;

    request.validate()?;
    let name = request.name.trim();
    let email = request.email.trim().to_lowercase();
    if name.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "name: is required"));
    }

    let user = user_queries::update_user(&state.pool, logged_in_user.id, name, &email)
        .await?
        .ok_or(ApiError::not_found())?;
    user_cache::remove_user(&state, logged_in_user.id).await;
    Ok(Json(user.to_response()))
}

// The app always needs one admin, so the last one cannot be deleted or made a normal user
async fn check_not_last_admin(state: &AppState, user_id: i32) -> Result<(), ApiError> {
    let user = match user_queries::get_user_by_id(&state.pool, user_id).await? {
        Some(user) => user,
        None => return Ok(()),
    };
    if user.role == "admin" && user_queries::count_admins(&state.pool).await? <= 1 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "cannot remove the last admin",
        ));
    }
    Ok(())
}
