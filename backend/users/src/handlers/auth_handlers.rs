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
    tag = "auth",
    summary = "Create an account",
    description = "Creates a new user. No token needed (public through the gateway).\n\n\
**Rules:** name 1 to 100 characters (not only spaces), a valid email that is not used yet (saved in lowercase), password 6 to 128 characters. \
The very first user becomes `admin` when no admin was created from ADMIN_EMAIL, everyone else gets `user`.\n\n\
After register the service sends a welcome message to the notifications service. If notifications is down, register still works.\n\n**Rate limit:** RATE_LIMIT_REQUESTS calls per IP in RATE_LIMIT_WINDOW_SECONDS (default 10 per 60 seconds), then 429 with a Retry-After header. Behind Traefik the IP is the last address in X-Forwarded-For.",
    request_body(content = RegisterRequest, description = "Name, email and password of the new account.", example = json!({ "name": "Test User", "email": "user@example.com", "password": "secret123" })),
    responses(
        (status = 201, description = "Account created. Log in next to get a token.", body = UserResponse, example = json!({ "id": 7, "name": "Test User", "email": "user@example.com", "role": "user", "createdAt": "2026-11-20T10:15:00Z" })),
        (status = 400, description = "Name, email or password breaks a rule. Several problems are joined with a comma.", body = ErrorResponse, examples(
            ("Short password" = (summary = "Password has fewer than 6 characters", value = json!({ "error": "password: must have 6 to 128 characters" }))),
            ("Bad email" = (summary = "Email is not valid", value = json!({ "error": "email: must be a valid email" }))),
            ("Bad name" = (summary = "Name is empty, only spaces or too long", value = json!({ "error": "name: must have 1 to 100 characters" }))),
            ("Bad JSON" = (summary = "The body is not valid JSON", value = json!({ "error": "Failed to parse the request body as JSON: key must be a string at line 1 column 2" }))),
            ("Only spaces" = (summary = "Name has only spaces", value = json!({ "error": "name: is required" })))
        )),
        (status = 409, description = "Another account already uses this email.", body = ErrorResponse, example = json!({ "error": "email already exists" })),
        (status = 415, description = "The body is not sent as JSON (missing `Content-Type: application/json`).", body = ErrorResponse, example = json!({ "error": "Expected request with `Content-Type: application/json`" })),
        (status = 422, description = "The JSON is valid but a field is missing or has the wrong type.", body = ErrorResponse, example = json!({ "error": "Failed to deserialize the JSON body into the target type: missing field `password` at line 1 column 2" })),
        (
            status = 429,
            description = "Rate limit: more than RATE_LIMIT_REQUESTS calls (default 10) from the same IP to this path in RATE_LIMIT_WINDOW_SECONDS (default 60). Wait the number of seconds in Retry-After.",
            body = ErrorResponse,
            headers(("Retry-After" = u64, description = "Seconds to wait before the next try")),
            example = json!({ "error": "too many requests, try again later" })
        ),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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
    tag = "auth",
    summary = "Log in",
    description = "Checks email and password and returns an access token, a refresh token and the user. No token needed (public through the gateway).\n\n\
The access token is a JWT valid for ACCESS_TOKEN_MINUTES (default 15). Send it as `Authorization: Bearer <token>` on protected calls. \
The refresh token is valid 7 days and works once, use it with POST /users/refresh.\n\n**Rate limit:** RATE_LIMIT_REQUESTS calls per IP in RATE_LIMIT_WINDOW_SECONDS (default 10 per 60 seconds), then 429 with a Retry-After header. Behind Traefik the IP is the last address in X-Forwarded-For.",
    request_body(content = LoginRequest, description = "Email and password.", example = json!({ "email": "user@example.com", "password": "secret123" })),
    responses(
        (status = 200, description = "Logged in.", body = LoginResponse),
        (status = 400, description = "The body is not valid JSON.", body = ErrorResponse, example = json!({ "error": "Failed to parse the request body as JSON: key must be a string at line 1 column 2" })),
        (status = 401, description = "No account with this email, or the password is wrong. Both give the same message on purpose.", body = ErrorResponse, example = json!({ "error": "wrong email or password" })),
        (status = 415, description = "The body is not sent as JSON (missing `Content-Type: application/json`).", body = ErrorResponse, example = json!({ "error": "Expected request with `Content-Type: application/json`" })),
        (status = 422, description = "The JSON is valid but a field is missing or has the wrong type.", body = ErrorResponse, example = json!({ "error": "Failed to deserialize the JSON body into the target type: missing field `password` at line 1 column 2" })),
        (
            status = 429,
            description = "Rate limit: more than RATE_LIMIT_REQUESTS calls (default 10) from the same IP to this path in RATE_LIMIT_WINDOW_SECONDS (default 60). Wait the number of seconds in Retry-After.",
            body = ErrorResponse,
            headers(("Retry-After" = u64, description = "Seconds to wait before the next try")),
            example = json!({ "error": "too many requests, try again later" })
        ),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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
    tag = "auth",
    summary = "Get new tokens with a refresh token",
    description = "Trades a refresh token for a new access token and a new refresh token. No access token needed (public through the gateway).\n\n\
Each refresh token works only once: after this call the old one is deleted. Refresh tokens are valid 7 days and are removed on logout, password change and when the user is deleted.\n\n**Rate limit:** RATE_LIMIT_REQUESTS calls per IP in RATE_LIMIT_WINDOW_SECONDS (default 10 per 60 seconds), then 429 with a Retry-After header. Behind Traefik the IP is the last address in X-Forwarded-For.",
    request_body(content = RefreshRequest, description = "The refresh token from login or from the last refresh."),
    responses(
        (status = 200, description = "New tokens, the old refresh token stops working.", body = RefreshResponse),
        (status = 400, description = "The body is not valid JSON.", body = ErrorResponse, example = json!({ "error": "Failed to parse the request body as JSON: key must be a string at line 1 column 2" })),
        (status = 401, description = "The refresh token is unknown, already used, expired, or its user was deleted.", body = ErrorResponse, example = json!({ "error": "invalid refresh token" })),
        (status = 415, description = "The body is not sent as JSON (missing `Content-Type: application/json`).", body = ErrorResponse, example = json!({ "error": "Expected request with `Content-Type: application/json`" })),
        (status = 422, description = "The JSON is valid but a field is missing or has the wrong type.", body = ErrorResponse, example = json!({ "error": "Failed to deserialize the JSON body into the target type: missing field `refreshToken` at line 1 column 2" })),
        (
            status = 429,
            description = "Rate limit: more than RATE_LIMIT_REQUESTS calls (default 10) from the same IP to this path in RATE_LIMIT_WINDOW_SECONDS (default 60). Wait the number of seconds in Retry-After.",
            body = ErrorResponse,
            headers(("Retry-After" = u64, description = "Seconds to wait before the next try")),
            example = json!({ "error": "too many requests, try again later" })
        ),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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
    tag = "auth",
    summary = "Log out everywhere",
    description = "Logs a user out on all devices: all access tokens issued before this call stop working and all refresh tokens are deleted.\n\n\
**Who can call it:** the user themselves, or an admin for any user. Send `Authorization: Bearer <token>` from login.",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "User id", example = 7)),
    responses(
        (status = 204, description = "Logged out. No body."),
        (status = 400, description = "The id in the path is not a whole number.", body = ErrorResponse, example = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })),
        (status = 401, description = "The bearer token is missing, expired, invalid, or was logged out (logout, password change and deleting the user make old tokens stop working).", body = ErrorResponse, example = json!({ "error": "missing or invalid token" })),
        (status = 403, description = "The id is not your own and you are not an admin.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 404, description = "No user with this id.", body = ErrorResponse, example = json!({ "error": "user not found" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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
    summary = "Check a token (used by Traefik)",
    description = "Traefik ForwardAuth calls this before every protected request through the gateway. \
When the bearer token is valid it answers 200 with the X-User-Id and X-User-Role headers, which Traefik copies to the target service. Otherwise it answers 401 and Traefik returns that 401 to the client.\n\n\
The token is checked for signature, expiry, an existing user and the current token version (logout and password change make old tokens invalid). \
Not routed through the gateway, it is only called inside docker.",
    security(("bearerAuth" = [])),
    responses(
        (
            status = 200,
            description = "Token is valid. No body.",
            headers(
                ("X-User-Id" = i32, description = "Id of the logged in user, for example 7"),
                ("X-User-Role" = String, description = "`user` or `admin`")
            )
        ),
        (status = 401, description = "The bearer token is missing, expired, invalid, or was logged out (logout, password change and deleting the user make old tokens stop working).", body = ErrorResponse, example = json!({ "error": "missing or invalid token" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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
    tag = "me",
    summary = "Change my password",
    description = "Changes the password of the logged in user. The old password must be correct and the new one must have 6 to 128 characters.\n\n\
After the change all refresh tokens are deleted and old access tokens stop working, so log in again with the new password.\n\n\
**Who can call it:** any logged in user, only for their own account. Send `Authorization: Bearer <token>` from login.",
    security(("bearerAuth" = [])),
    request_body(content = ChangePasswordRequest, description = "Old and new password.", example = json!({ "oldPassword": "secret123", "newPassword": "newsecret123" })),
    responses(
        (status = 204, description = "Password changed. No body."),
        (status = 400, description = "The new password is too short or too long, or the body is not valid JSON.", body = ErrorResponse, example = json!({ "error": "newPassword: must have 6 to 128 characters" })),
        (status = 401, description = "The token is missing or invalid, or the old password is wrong.", body = ErrorResponse, examples(
            ("Bad token" = (summary = "Missing or invalid token", value = json!({ "error": "missing or invalid token" }))),
            ("Wrong old password" = (summary = "oldPassword does not match", value = json!({ "error": "old password is wrong" })))
        )),
        (status = 415, description = "The body is not sent as JSON (missing `Content-Type: application/json`).", body = ErrorResponse, example = json!({ "error": "Expected request with `Content-Type: application/json`" })),
        (status = 422, description = "The JSON is valid but a field is missing or has the wrong type.", body = ErrorResponse, example = json!({ "error": "Failed to deserialize the JSON body into the target type: missing field `newPassword` at line 1 column 2" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
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
