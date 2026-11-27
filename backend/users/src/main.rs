mod auth;
mod cache;
mod clients;
mod config;
mod controllers;
mod db;
mod errors;
mod extractors;
mod models;
mod openapi;
mod rate_limit;
mod routes;
mod services;
mod state;

#[cfg(test)]
mod endpoint_tests;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use config::Config;
use rate_limit::RateLimiter;
use state::AppState;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Logs go to stdout, change the level with RUST_LOG (default info)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .init();

    let config = Config::from_env();

    let pool = db::connect(&config.database_url).await;
    db::create_tables(&pool).await;
    if let (Some(admin_email), Some(admin_password)) = (&config.admin_email, &config.admin_password)
    {
        services::user_service::seed_admin(&pool, admin_email, admin_password).await;
    }

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("could not create http client");

    let cache = cache::connect(&config.cache_url).await;

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        notifications_url: config.notifications_url,
        access_token_minutes: config.access_token_minutes,
        http_client,
        cache,
        cache_ttl_seconds: config.cache_ttl_seconds,
        rate_limiter: Arc::new(RateLimiter::new(
            config.rate_limit_requests,
            config.rate_limit_window_seconds,
        )),
    };

    let app = routes::create_router(state);

    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    println!("users service running on {address}");
    // connect info gives the client address to the rate limit
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
