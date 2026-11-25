use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

// The validate rules are checked in the handlers with request.validate(),
// the schema rules only show the same limits in Swagger

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    #[validate(length(min = 1, max = 100, message = "must have 1 to 100 characters"))]
    #[schema(min_length = 1, max_length = 100, example = "Ana Novak")]
    pub name: String,
    #[validate(email(message = "must be a valid email"))]
    #[schema(format = Email, example = "ana@example.com")]
    pub email: String,
    #[validate(length(min = 6, max = 128, message = "must have 6 to 128 characters"))]
    #[schema(min_length = 6, max_length = 128, example = "secret123")]
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[schema(example = "ana@example.com")]
    pub email: String,
    #[schema(example = "secret123")]
    pub password: String,
}

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[schema(example = "secret123")]
    pub old_password: String,
    #[validate(length(min = 6, max = 128, message = "must have 6 to 128 characters"))]
    #[schema(min_length = 6, max_length = 128, example = "newsecret123")]
    pub new_password: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    #[validate(length(min = 1, max = 100, message = "must have 1 to 100 characters"))]
    #[schema(min_length = 1, max_length = 100, example = "Ana Novak")]
    pub name: String,
    #[validate(email(message = "must be a valid email"))]
    #[schema(format = Email, example = "ana@example.com")]
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
    /// "user" or "admin"
    #[schema(example = "admin")]
    pub role: String,
}

// ?page=1&pageSize=20 on GET /users
#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct PageQuery {
    /// Page number, starts at 1 (default 1)
    #[param(minimum = 1)]
    pub page: Option<i64>,
    /// Users per page, 1 to 100 (default 20)
    #[param(minimum = 1, maximum = 100)]
    pub page_size: Option<i64>,
}
