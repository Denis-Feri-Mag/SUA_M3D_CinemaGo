use axum::http::HeaderMap;

use crate::errors::ApiError;

// Traefik checks the token with the users service and then sends
// X-User-Id and X-User-Role to this service, so we only read the headers
pub struct CurrentUser {
    pub id: i32,
    pub role: String,
}

impl CurrentUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
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

// The owner of the reservation or an admin
pub fn check_owner_or_admin(user: &CurrentUser, owner_id: i32) -> Result<(), ApiError> {
    if user.id == owner_id || user.is_admin() {
        Ok(())
    } else {
        Err(ApiError::forbidden())
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
    }

    #[test]
    fn rejects_missing_or_bad_headers() {
        assert!(from_headers(&HeaderMap::new()).is_err());
        assert!(from_headers(&headers("abc", "user")).is_err());
    }

    #[test]
    fn owner_and_admin_are_allowed() {
        let owner = from_headers(&headers("7", "user")).unwrap();
        let other_user = from_headers(&headers("8", "user")).unwrap();
        let admin = from_headers(&headers("1", "admin")).unwrap();
        assert!(check_owner_or_admin(&owner, 7).is_ok());
        assert!(check_owner_or_admin(&other_user, 7).is_err());
        assert!(check_owner_or_admin(&admin, 7).is_ok());
    }
}
