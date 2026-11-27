// Business rules for user accounts: reading, changing, roles, deleting and the admin seed.

use axum::http::StatusCode;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::password;
use crate::cache::user_cache;
use crate::db::user_repository;
use crate::errors::ApiError;
use crate::models::requests::{PageQuery, UpdateRoleRequest, UpdateUserRequest};
use crate::models::responses::{UserPage, UserResponse};
use crate::models::user::{CachedUser, ROLE_ADMIN, ROLE_USER};
use crate::state::AppState;

const DEFAULT_PAGE_SIZE: i64 = 20;
const MAX_PAGE_SIZE: i64 = 100;

// The query is passed as a Result so a non admin gets 403 before a bad query gives 400
pub async fn list_users(
    state: &AppState,
    logged_in_user: &CachedUser,
    page_query: Result<PageQuery, ApiError>,
) -> Result<UserPage, ApiError> {
    check_admin(logged_in_user)?;
    let page_query = page_query?;

    let page = page_query.page.unwrap_or(1);
    let page_size = page_query.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    if page < 1 || !(1..=MAX_PAGE_SIZE).contains(&page_size) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "page must be at least 1 and pageSize 1 to 100",
        ));
    }

    let offset = (page - 1) * page_size;
    let users = user_repository::get_users_page(&state.pool, page_size, offset).await?;
    let total = user_repository::count_users(&state.pool).await?;
    let mut user_list = Vec::new();
    for user in users {
        user_list.push(user.to_response());
    }
    Ok(UserPage {
        items: user_list,
        page,
        page_size,
        total,
    })
}

// No token needed, other services use this to check a user exists
pub async fn get_user(state: &AppState, user_id: i32) -> Result<UserResponse, ApiError> {
    let user = user_cache::find_user(state, user_id)
        .await?
        .ok_or(ApiError::not_found())?;
    Ok(user.to_response())
}

pub async fn update_user(
    state: &AppState,
    logged_in_user: &CachedUser,
    user_id: i32,
    request: UpdateUserRequest,
) -> Result<UserResponse, ApiError> {
    check_self_or_admin(logged_in_user, user_id)?;
    save_name_and_email(state, user_id, request).await
}

pub async fn update_me(
    state: &AppState,
    logged_in_user: &CachedUser,
    request: UpdateUserRequest,
) -> Result<UserResponse, ApiError> {
    save_name_and_email(state, logged_in_user.id, request).await
}

pub async fn update_role(
    state: &AppState,
    logged_in_user: &CachedUser,
    user_id: i32,
    request: UpdateRoleRequest,
) -> Result<UserResponse, ApiError> {
    check_admin(logged_in_user)?;

    if request.role != ROLE_USER && request.role != ROLE_ADMIN {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "role must be user or admin",
        ));
    }
    if request.role == ROLE_USER {
        check_not_last_admin(state, user_id).await?;
    }

    let user = user_repository::update_role(&state.pool, user_id, &request.role)
        .await?
        .ok_or(ApiError::not_found())?;
    user_cache::remove_user(state, user_id).await;
    Ok(user.to_response())
}

pub async fn delete_user(
    state: &AppState,
    logged_in_user: &CachedUser,
    user_id: i32,
) -> Result<(), ApiError> {
    check_self_or_admin(logged_in_user, user_id)?;
    check_not_last_admin(state, user_id).await?;

    let was_deleted = user_repository::delete_user(&state.pool, user_id).await?;
    user_cache::remove_user(state, user_id).await;
    if !was_deleted {
        return Err(ApiError::not_found());
    }
    Ok(())
}

// Creates the admin from ADMIN_EMAIL and ADMIN_PASSWORD on start if it does not exist yet.
// If the user already exists nothing changes (also not the password).
pub async fn seed_admin(pool: &PgPool, admin_email: &str, admin_password: &str) {
    let email = admin_email.trim().to_lowercase();
    let existing_user = user_repository::get_user_by_email(pool, &email)
        .await
        .expect("could not check the admin user");
    if existing_user.is_some() {
        return;
    }

    let password_hash = password::hash_password(admin_password);
    user_repository::insert_user(pool, "Admin", &email, &password_hash, ROLE_ADMIN)
        .await
        .expect("could not create the admin user");
    tracing::info!("created admin user {email}");
}

// A user can change their own account, an admin can change any account
pub fn check_self_or_admin(logged_in_user: &CachedUser, user_id: i32) -> Result<(), ApiError> {
    if logged_in_user.id == user_id || logged_in_user.role == ROLE_ADMIN {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}

fn check_admin(logged_in_user: &CachedUser) -> Result<(), ApiError> {
    if logged_in_user.role != ROLE_ADMIN {
        return Err(ApiError::forbidden());
    }
    Ok(())
}

// Trims both, saves the email in lowercase.
// Only spaces passes the length check, so the trimmed name is checked too.
pub fn clean_name_and_email(name: &str, email: &str) -> Result<(String, String), ApiError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "name: is required"));
    }
    Ok((name.to_string(), email.trim().to_lowercase()))
}

async fn save_name_and_email(
    state: &AppState,
    user_id: i32,
    request: UpdateUserRequest,
) -> Result<UserResponse, ApiError> {
    request.validate()?;
    let (name, email) = clean_name_and_email(&request.name, &request.email)?;

    let user = user_repository::update_user(&state.pool, user_id, &name, &email)
        .await?
        .ok_or(ApiError::not_found())?;
    user_cache::remove_user(state, user_id).await;
    Ok(user.to_response())
}

// The app always needs one admin, so the last one cannot be deleted or made a normal user
async fn check_not_last_admin(state: &AppState, user_id: i32) -> Result<(), ApiError> {
    let user = match user_repository::get_user_by_id(&state.pool, user_id).await? {
        Some(user) => user,
        None => return Ok(()),
    };
    if user.role == ROLE_ADMIN && user_repository::count_admins(&state.pool).await? <= 1 {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "cannot remove the last admin",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_with_role(id: i32, role: &str) -> CachedUser {
        CachedUser {
            id,
            name: "Test User".to_string(),
            email: "user@example.com".to_string(),
            role: role.to_string(),
            token_version: 0,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn self_and_admin_can_change_an_account() {
        assert!(check_self_or_admin(&user_with_role(7, ROLE_USER), 7).is_ok());
        assert!(check_self_or_admin(&user_with_role(1, ROLE_ADMIN), 7).is_ok());
        assert!(check_self_or_admin(&user_with_role(8, ROLE_USER), 7).is_err());
    }

    #[test]
    fn only_admin_passes_the_admin_check() {
        assert!(check_admin(&user_with_role(1, ROLE_ADMIN)).is_ok());
        assert!(check_admin(&user_with_role(7, ROLE_USER)).is_err());
    }

    #[test]
    fn name_and_email_are_cleaned() {
        let cleaned = clean_name_and_email("  Test User ", " User@Example.COM ");
        assert!(
            matches!(cleaned, Ok((name, email)) if name == "Test User" && email == "user@example.com")
        );
        assert!(clean_name_and_email("   ", "user@example.com").is_err());
    }
}
