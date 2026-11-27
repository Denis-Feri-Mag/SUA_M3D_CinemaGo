mod auth;
mod clients;
mod config;
mod controllers;
mod db;
mod errors;
mod extractors;
mod models;
mod openapi;
mod routes;
mod services;
mod state;

#[cfg(test)]
mod endpoint_tests;

use std::time::Duration;

use config::Config;
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

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("could not create http client");

    let state = AppState {
        pool,
        screenings_url: config.screenings_url,
        http_client,
    };

    let app = routes::create_router(state);

    let address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    println!("reservations service running on {address}");
    axum::serve(listener, app).await.unwrap();
}
