use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::models::responses::ErrorResponse;

// Every handler returns this on failure, it becomes { "error": "..." }
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: &str) -> ApiError {
        ApiError {
            status,
            message: message.to_string(),
        }
    }

    pub fn not_found() -> ApiError {
        ApiError::new(StatusCode::NOT_FOUND, "user not found")
    }

    pub fn unauthorized() -> ApiError {
        ApiError::new(StatusCode::UNAUTHORIZED, "missing or invalid token")
    }

    pub fn forbidden() -> ApiError {
        ApiError::new(StatusCode::FORBIDDEN, "not allowed")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorResponse {
            error: self.message,
        };
        (self.status, Json(body)).into_response()
    }
}

// Lets us use ? on database calls
impl From<sqlx::Error> for ApiError {
    fn from(database_error: sqlx::Error) -> ApiError {
        if let Some(error_details) = database_error.as_database_error() {
            if error_details.is_unique_violation() {
                return ApiError::new(StatusCode::CONFLICT, "email already exists");
            }
        }
        eprintln!("database error: {database_error}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "database error")
    }
}
