use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// A user account. The password hash is never returned.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    /// User id. Other services get it in the X-User-Id header.
    #[schema(example = 7)]
    pub id: i32,
    /// Display name.
    #[schema(example = "Test User")]
    pub name: String,
    /// Email in lowercase.
    #[schema(example = "user@example.com")]
    pub email: String,
    /// `user` or `admin`.
    #[schema(example = "user")]
    pub role: String,
    /// When the account was created (UTC).
    #[schema(example = "2026-11-20T10:15:00Z")]
    pub created_at: DateTime<Utc>,
}

/// One page of users.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserPage {
    /// Users on this page, sorted by id.
    pub items: Vec<UserResponse>,
    /// Page number that was returned.
    #[schema(example = 1)]
    pub page: i64,
    /// Page size that was used.
    #[schema(example = 20)]
    pub page_size: i64,
    /// Number of all users on all pages.
    #[schema(example = 1)]
    pub total: i64,
}

/// Tokens and the user after a successful login.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    /// Access token (JWT), valid for ACCESS_TOKEN_MINUTES (default 15). Send it as `Authorization: Bearer <token>`.
    #[schema(example = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI3Iiwicm9sZSI6InVzZXIifQ.signature")]
    pub token: String,
    /// Refresh token, valid 7 days and only once. Send it to POST /users/refresh when the access token expires.
    #[schema(example = "6f1c2a9e0b7d4c3fa8e5d2b1c0f9e8d76a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d")]
    pub refresh_token: String,
    /// The logged in user.
    pub user: UserResponse,
}

/// New tokens after a refresh.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    /// New access token (JWT).
    #[schema(example = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI3Iiwicm9sZSI6InVzZXIifQ.signature")]
    pub token: String,
    /// New refresh token, the old one no longer works.
    #[schema(example = "0a9b8c7d6e5f4a3b2c1d6f1c2a9e0b7d4c3fa8e5d2b1c0f9e8d76a5b4c3d2e1f")]
    pub refresh_token: String,
}

/// Health check answer.
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    /// Always `ok` when the service is running.
    #[schema(example = "ok")]
    pub status: String,
}

/// Every error in this service has this shape.
#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Short message that says what went wrong.
    #[schema(example = "user not found")]
    pub error: String,
}
