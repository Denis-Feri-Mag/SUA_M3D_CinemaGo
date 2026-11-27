use redis::AsyncCommands;

use crate::db::user_repository;
use crate::models::user::CachedUser;
use crate::state::AppState;

// One key per user, like "users:user:5"
fn user_key(user_id: i32) -> String {
    format!("users:user:{user_id}")
}

// Looks in the cache first, then in the database (and saves the result in the cache)
pub async fn find_user(state: &AppState, user_id: i32) -> Result<Option<CachedUser>, sqlx::Error> {
    if let Some(cached_user) = read_from_cache(state, user_id).await {
        return Ok(Some(cached_user));
    }

    let user = match user_repository::get_user_by_id(&state.pool, user_id).await? {
        Some(user_row) => user_row.to_cached_user(),
        None => return Ok(None),
    };
    save_to_cache(state, &user).await;
    Ok(Some(user))
}

// Call this after a user is changed or deleted, so the next read comes from the database
pub async fn remove_user(state: &AppState, user_id: i32) {
    let Some(mut connection) = state.cache.clone() else {
        return;
    };
    let result: redis::RedisResult<()> = connection.del(user_key(user_id)).await;
    if let Err(error) = result {
        eprintln!("warning: could not remove user {user_id} from cache: {error}");
    }
}

async fn read_from_cache(state: &AppState, user_id: i32) -> Option<CachedUser> {
    let mut connection = state.cache.clone()?;
    let result: redis::RedisResult<Option<String>> = connection.get(user_key(user_id)).await;
    match result {
        Ok(Some(json_text)) => serde_json::from_str(&json_text).ok(),
        Ok(None) => None,
        Err(error) => {
            eprintln!("warning: cache read failed, using database: {error}");
            None
        }
    }
}

async fn save_to_cache(state: &AppState, user: &CachedUser) {
    let Some(mut connection) = state.cache.clone() else {
        return;
    };
    let Ok(json_text) = serde_json::to_string(user) else {
        return;
    };
    // The entry is deleted automatically after CACHE_TTL_SECONDS
    let result: redis::RedisResult<()> = connection
        .set_ex(user_key(user.id), json_text, state.cache_ttl_seconds)
        .await;
    if let Err(error) = result {
        eprintln!("warning: cache write failed: {error}");
    }
}
