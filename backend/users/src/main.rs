mod auth;
mod cache;
mod clients;
mod config;
mod db;
mod errors;
mod handlers;
mod models;
mod openapi;
mod routes;
mod state;

#[cfg(test)]
mod endpoint_tests;

use std::time::Duration;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() {
    let config = Config::from_env();

    let pool = db::connect(&config.database_url).await;
    db::create_tables(&pool).await;

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
    };

    let app = routes::create_router(state);

    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    println!("users service running on {address}");
    axum::serve(listener, app).await.unwrap();
}
