use sqlx::PgPool;

// Shared by all handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub screenings_url: String,
    pub http_client: reqwest::Client,
}
