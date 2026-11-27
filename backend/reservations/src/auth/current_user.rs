use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderMap;

use crate::errors::ApiError;

pub const ADMIN_ROLE: &str = "admin";

// Traefik checks the token with the users service and then sends
// X-User-Id and X-User-Role to this service, so we only read the headers
pub struct CurrentUser {
    pub id: i32,
    pub role: String,
}

impl CurrentUser {
    pub fn is_admin(&self) -> bool {
        self.role == ADMIN_ROLE
    }
}

pub fn from_headers(headers: &HeaderMap) -> Result<CurrentUser, ApiError> {
    let user_id = headers
        .get("X-User-Id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<i32>().ok())
        .ok_or(ApiError::unauthorized())?;

    let role = headers
        .get("X-User-Role")
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::unauthorized())?;

    Ok(CurrentUser {
        id: user_id,
        role: role.to_string(),
    })
}

// Lets a controller take `current_user: CurrentUser` as an argument,
// a missing or bad header becomes 401 before the controller runs
impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        from_headers(&parts.headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(user_id: &str, role: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("X-User-Id", user_id.parse().unwrap());
        headers.insert("X-User-Role", role.parse().unwrap());
        headers
    }

    #[test]
    fn reads_user_from_headers() {
        let user = from_headers(&headers("7", "user")).unwrap();
        assert_eq!(user.id, 7);
        assert!(!user.is_admin());
        assert!(from_headers(&headers("1", "admin")).unwrap().is_admin());
    }

    #[test]
    fn rejects_missing_or_bad_headers() {
        assert!(from_headers(&HeaderMap::new()).is_err());
        assert!(from_headers(&headers("abc", "user")).is_err());
    }
}
