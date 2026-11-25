use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

// What we keep in the cache, without the password hash
#[derive(Serialize, Deserialize)]
pub struct CachedUser {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub role: String,
    pub token_version: i32,
    pub created_at: DateTime<Utc>,
}

impl CachedUser {
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

impl UserRow {
    pub fn to_cached_user(&self) -> CachedUser {
        CachedUser {
            id: self.id,
            name: self.name.clone(),
            email: self.email.clone(),
            role: self.role.clone(),
            token_version: self.token_version,
            created_at: self.created_at,
        }
    }

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
