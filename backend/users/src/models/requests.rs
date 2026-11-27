use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

// The validate rules are checked in the handlers with request.validate(),
// the schema rules only show the same limits in Swagger

/// Body for creating a new account.
#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    /// Display name, 1 to 100 characters. Spaces at the start and end are removed, only spaces is not allowed.
    #[validate(length(min = 1, max = 100, message = "must have 1 to 100 characters"))]
    #[schema(min_length = 1, max_length = 100, example = "Test User")]
    pub name: String,
    /// Email used to log in. Must be valid and not used by another account, saved in lowercase.
    #[validate(email(message = "must be a valid email"))]
    #[schema(format = Email, example = "user@example.com")]
    pub email: String,
    /// Password, 6 to 128 characters. Only a hash is saved.
    #[validate(length(min = 6, max = 128, message = "must have 6 to 128 characters"))]
    #[schema(min_length = 6, max_length = 128, example = "secret123")]
    pub password: String,
}

/// Body for logging in.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    /// Email of the account, upper or lower case does not matter.
    #[schema(example = "user@example.com")]
    pub email: String,
    /// Password of the account.
    #[schema(example = "secret123")]
    pub password: String,
}

/// Body for changing the own password.
#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    /// The current password.
    #[schema(example = "secret123")]
    pub old_password: String,
    /// The new password, 6 to 128 characters.
    #[validate(length(min = 6, max = 128, message = "must have 6 to 128 characters"))]
    #[schema(min_length = 6, max_length = 128, example = "newsecret123")]
    pub new_password: String,
}

/// Body for getting new tokens.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    /// Refresh token from login or from the last refresh. Each one works only once.
    #[schema(example = "6f1c2a9e0b7d4c3fa8e5d2b1c0f9e8d76a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d")]
    pub refresh_token: String,
}

/// Body for changing name and email. Both fields are required.
#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    /// New display name, 1 to 100 characters, not only spaces.
    #[validate(length(min = 1, max = 100, message = "must have 1 to 100 characters"))]
    #[schema(min_length = 1, max_length = 100, example = "Test User")]
    pub name: String,
    /// New email, must be valid and not used by another account. Saved in lowercase.
    #[validate(email(message = "must be a valid email"))]
    #[schema(format = Email, example = "user@example.com")]
    pub email: String,
}

/// Body for changing the role of a user.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
    /// New role, `user` or `admin`.
    #[schema(example = "admin")]
    pub role: String,
}

// ?page=1&pageSize=20 on GET /users
#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct PageQuery {
    /// Page number, starts at 1. Default 1.
    #[param(minimum = 1, example = 1)]
    pub page: Option<i64>,
    /// Users per page, 1 to 100. Default 20.
    #[param(minimum = 1, maximum = 100, example = 20)]
    pub page_size: Option<i64>,
}
