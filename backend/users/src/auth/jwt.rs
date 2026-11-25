use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::models::user::UserRow;

// Data stored inside the JWT
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub sub: String,
    pub name: String,
    pub role: String,
    pub token_version: i32,
    pub iat: usize,
    pub exp: usize,
}

pub fn create_token(user: &UserRow, jwt_secret: &str, valid_minutes: i64) -> String {
    // Short lived access token, the client gets a new one with the refresh token
    let issued_at = chrono::Utc::now();
    let expires_at = issued_at + chrono::Duration::minutes(valid_minutes);
    let claims = Claims {
        sub: user.id.to_string(),
        name: user.name.clone(),
        role: user.role.clone(),
        token_version: user.token_version,
        iat: issued_at.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .expect("could not create token")
}

// Returns the claims if the token is signed with our secret and not expired
pub fn read_token(token: &str, jwt_secret: &str) -> Option<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?;
    Some(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_user() -> UserRow {
        UserRow {
            id: 7,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            role: "admin".to_string(),
            token_version: 3,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn created_token_can_be_read() {
        let token = create_token(&test_user(), "test-secret", 15);
        let claims = read_token(&token, "test-secret").expect("token should be valid");
        assert_eq!(claims.sub, "7");
        assert_eq!(claims.role, "admin");
        assert_eq!(claims.token_version, 3);
        assert_eq!(claims.exp - claims.iat, 15 * 60);
    }

    #[test]
    fn token_with_other_secret_is_rejected() {
        let token = create_token(&test_user(), "test-secret", 15);
        assert!(read_token(&token, "other-secret").is_none());
    }

    #[test]
    fn expired_token_is_rejected() {
        // Expired 10 minutes ago, more than the 60 second leeway
        let issued_at = chrono::Utc::now() - chrono::Duration::minutes(30);
        let claims = Claims {
            sub: "7".to_string(),
            name: "Test User".to_string(),
            role: "user".to_string(),
            token_version: 0,
            iat: issued_at.timestamp() as usize,
            exp: (issued_at + chrono::Duration::minutes(20)).timestamp() as usize,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("test-secret".as_bytes()),
        )
        .unwrap();
        assert!(read_token(&token, "test-secret").is_none());
    }

    #[test]
    fn broken_token_is_rejected() {
        assert!(read_token("not.a.token", "test-secret").is_none());
    }
}
