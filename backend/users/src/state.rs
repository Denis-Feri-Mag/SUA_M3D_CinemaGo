use redis::aio::ConnectionManager;
use sqlx::PgPool;

// Shared by all handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub notifications_url: String,
    pub access_token_minutes: i64,
    pub http_client: reqwest::Client,
    // None when Valkey is not running
    pub cache: Option<ConnectionManager>,
    pub cache_ttl_seconds: u64,
}
