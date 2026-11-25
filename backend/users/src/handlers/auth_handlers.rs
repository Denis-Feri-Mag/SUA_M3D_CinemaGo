use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;

use crate::auth::{current_user, jwt, password};
use crate::clients::notifications;
use crate::db::user_queries;
use crate::errors::ApiError;
use crate::models::requests::{LoginRequest, RegisterRequest};
use crate::models::responses::{ErrorResponse, LoginResponse, UserResponse};
use crate::state::AppState;

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
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "name and email are required",
        ));
    }
    if request.password.len() < 6 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "password must have at least 6 characters",
        ));
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

    let user = match user_queries::get_user_by_email(&state.pool, &email).await? {
        Some(user) => user,
        None => return Err(wrong_login),
    };

    if !password::verify_password(&request.password, &user.password_hash) {
        return Err(wrong_login);
    }

    let token = jwt::create_token(&user, &state.jwt_secret);
    Ok(Json(LoginResponse {
        token,
        user: user.to_response(),
    }))
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
    let logged_in_user = current_user::get_logged_in_user(&headers, &state).await?;
    current_user::check_self_or_admin(&logged_in_user, user_id)?;

    let was_updated = user_queries::increase_token_version(&state.pool, user_id).await?;
    if !was_updated {
        return Err(ApiError::not_found());
    }
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
