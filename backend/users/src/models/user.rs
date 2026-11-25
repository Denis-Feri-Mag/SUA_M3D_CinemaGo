use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::models::responses::UserResponse;

// One row from the users table
#[derive(FromRow)]
pub struct UserRow {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub token_version: i32,
    pub created_at: DateTime<Utc>,
}

impl UserRow {
    pub fn to_response(&self) -> UserResponse {
        UserResponse {
            id: self.id,
            name: self.name.clone(),
            email: self.email.clone(),
            role: self.role.clone(),
            created_at: self.created_at,
        }
    }
}
