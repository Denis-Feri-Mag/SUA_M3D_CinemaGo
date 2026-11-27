use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::handlers::{auth_handlers, health, user_handlers};

const API_DESCRIPTION: &str = "Users service of CinemaGo. Handles accounts, login with JWT access tokens and refresh tokens, roles, and the token check that the Traefik gateway runs before every protected request.

## How to log in

1. `POST /users/register` with name, email and password (or use the admin from ADMIN_EMAIL and ADMIN_PASSWORD).
2. `POST /users/login` returns `token` (access token, valid ACCESS_TOKEN_MINUTES, default 15) and `refreshToken` (valid 7 days, works once).
3. Send `Authorization: Bearer <token>` on protected calls (use the Authorize button).
4. When the access token expires, `POST /users/refresh` with the refresh token gives new tokens.

Logout (`DELETE /users/{id}/sessions`), a password change and deleting the user make all older tokens stop working.

## Gateway

Through Traefik (server `/api`, for example `http://localhost:8080/api/users/login`) register, login and refresh are public, everything else needs a token. \
For other services Traefik calls `GET /auth/verify` and passes `X-User-Id` and `X-User-Role` on.

## Roles

`user` can read and change only their own account. `admin` can list all users, change roles and change or delete any user. There is always at least one admin: the last admin cannot be deleted or made a user.

## Rate limit

Register, login and refresh allow RATE_LIMIT_REQUESTS calls (default 10) per client IP and path in RATE_LIMIT_WINDOW_SECONDS (default 60). After that they answer 429 with a `Retry-After` header in seconds.

## Validation

Name 1 to 100 characters (not only spaces), a valid email (saved in lowercase, unique), password 6 to 128 characters. Broken rules return 400 like `{\"error\": \"email: must be a valid email\"}`.

## Errors

Every error from this service is JSON like `{\"error\": \"user not found\"}`. This includes bad JSON bodies (400, 415, 422), bad path or query values (400) and unknown paths (404). When the service is down, the gateway itself answers 502 or 503 without a JSON body.";

#[derive(OpenApi)]
#[openapi(
    info(
        title = "CinemaGo users service",
        version = "1.0.0",
        description = API_DESCRIPTION
    ),
    servers(
        (url = "/", description = "Direct to the users service (dev port 8081)"),
        (url = "/api", description = "Through the Traefik gateway (port 8080)")
    ),
    tags(
        (name = "auth", description = "Register, login, refresh, logout and the token check for Traefik"),
        (name = "me", description = "The logged in user"),
        (name = "users", description = "Read, change and delete one user"),
        (name = "admin", description = "Only for admins"),
        (name = "health", description = "Service status")
    ),
    paths(
        health::health,
        user_handlers::get_users,
        user_handlers::get_user,
        user_handlers::get_me,
        user_handlers::update_me,
        auth_handlers::register,
        auth_handlers::login,
        auth_handlers::refresh,
        user_handlers::update_user,
        user_handlers::update_role,
        user_handlers::delete_user,
        auth_handlers::logout,
        auth_handlers::change_password,
        auth_handlers::verify
    ),
    modifiers(&BearerAuth)
)]
pub struct ApiDoc;

// Adds the "Authorize" button for JWT in Swagger
struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // No license in Cargo.toml, so leave it out instead of an empty one
        openapi.info.license = None;
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some(
                            "Access token from POST /users/login or POST /users/refresh, valid ACCESS_TOKEN_MINUTES (default 15).",
                        ))
                        .build(),
                ),
            );
        }
    }
}
