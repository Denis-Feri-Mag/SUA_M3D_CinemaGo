use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::models::user::UserRow;

// Data stored inside the JWT
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub token_version: i32,
    pub exp: usize,
}

pub fn create_token(user: &UserRow, jwt_secret: &str) -> String {
    // Token is valid for 24 hours
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);
    let claims = Claims {
        sub: user.id.to_string(),
        role: user.role.clone(),
        token_version: user.token_version,
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
