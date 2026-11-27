use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(FromRow)]
pub struct RefreshTokenRow {
    pub user_id: i32,
    pub expires_at: DateTime<Utc>,
}

pub async fn insert_refresh_token(
    pool: &PgPool,
    user_id: i32,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(())
}

// Deletes the token and returns it, so each refresh token works only once
pub async fn take_refresh_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<RefreshTokenRow>, sqlx::Error> {
    sqlx::query_as::<_, RefreshTokenRow>(
        "DELETE FROM refresh_tokens WHERE token_hash = $1 RETURNING user_id, expires_at",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

pub async fn delete_refresh_tokens_for_user(
    pool: &PgPool,
    user_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
