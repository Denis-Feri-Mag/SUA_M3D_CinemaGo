use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

// What we send back to the client (no password hash)
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

// One page of GET /users
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserPage {
    pub items: Vec<UserResponse>,
    pub page: i64,
    pub page_size: i64,
    /// Number of all users
    pub total: i64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserResponse,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub token: String,
    pub refresh_token: String,
}

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}
