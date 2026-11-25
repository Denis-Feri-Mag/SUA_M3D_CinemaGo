use sqlx::PgPool;

use crate::models::user::UserRow;

pub async fn get_all_users(pool: &PgPool) -> Result<Vec<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn get_user_by_id(pool: &PgPool, user_id: i32) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
}

pub async fn count_users(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
}

pub async fn insert_user(
    pool: &PgPool,
    name: &str,
    email: &str,
    password_hash: &str,
    role: &str,
) -> Result<UserRow, sqlx::Error> {
    sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (name, email, password_hash, role) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .fetch_one(pool)
    .await
}

pub async fn update_user(
    pool: &PgPool,
    user_id: i32,
    name: &str,
    email: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>("UPDATE users SET name = $1, email = $2 WHERE id = $3 RETURNING *")
        .bind(name)
        .bind(email)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn update_role(
    pool: &PgPool,
    user_id: i32,
    role: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>("UPDATE users SET role = $1 WHERE id = $2 RETURNING *")
        .bind(role)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

// Returns true if a row was deleted
pub async fn delete_user(pool: &PgPool, user_id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// Old tokens stop working because their version no longer matches
pub async fn increase_token_version(pool: &PgPool, user_id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE users SET token_version = token_version + 1 WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
