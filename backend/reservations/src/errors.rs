use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::models::responses::ErrorResponse;

// Every handler returns this on failure, it becomes { "error": "..." }
#[derive(Debug)]
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
        ApiError::new(StatusCode::NOT_FOUND, "reservation not found")
    }

    pub fn unauthorized() -> ApiError {
        ApiError::new(StatusCode::UNAUTHORIZED, "missing or invalid user")
    }

    pub fn forbidden() -> ApiError {
        ApiError::new(StatusCode::FORBIDDEN, "not allowed")
    }

    pub fn bad_request(message: &str) -> ApiError {
        ApiError::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn conflict(message: &str) -> ApiError {
        ApiError::new(StatusCode::CONFLICT, message)
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

// Lets us use ? on request.validate(), for example "seats: must have 1 to 10 seats"
impl From<validator::ValidationErrors> for ApiError {
    fn from(validation_errors: validator::ValidationErrors) -> ApiError {
        let mut messages = Vec::new();
        for (field, field_errors) in validation_errors.field_errors() {
            for field_error in field_errors {
                let text = match &field_error.message {
                    Some(message) => message.to_string(),
                    None => "is not valid".to_string(),
                };
                messages.push(format!("{}: {text}", camel_case(&field)));
            }
        }
        messages.sort();
        ApiError::bad_request(&messages.join(", "))
    }
}

// screening_id becomes screeningId, like the field names in the JSON body
fn camel_case(field: &str) -> String {
    let mut result = String::new();
    let mut next_upper = false;
    for character in field.chars() {
        if character == '_' {
            next_upper = true;
        } else if next_upper {
            result.push(character.to_ascii_uppercase());
            next_upper = false;
        } else {
            result.push(character);
        }
    }
    result
}

// Bad query string, for example ?page=abc
impl From<axum::extract::rejection::QueryRejection> for ApiError {
    fn from(rejection: axum::extract::rejection::QueryRejection) -> ApiError {
        ApiError::bad_request(&rejection.body_text())
    }
}

// Lets us use ? on database calls
impl From<sqlx::Error> for ApiError {
    fn from(database_error: sqlx::Error) -> ApiError {
        if let Some(error_details) = database_error.as_database_error() {
            // The only unique index is the one on taken seats
            if error_details.is_unique_violation() {
                return ApiError::conflict("seat already taken");
            }
        }
        eprintln!("database error: {database_error}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "database error")
    }
}

#[cfg(test)]
mod tests {
    use super::camel_case;

    #[test]
    fn field_names_are_camel_case() {
        assert_eq!(camel_case("screening_id"), "screeningId");
        assert_eq!(camel_case("seats"), "seats");
    }
}
