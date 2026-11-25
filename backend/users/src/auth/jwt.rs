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
