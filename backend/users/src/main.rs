mod auth;
mod db;
mod errors;
mod handlers;
mod models;
mod notifications;

use std::env;
use std::time::Duration;

use axum::routing::{delete, get, post, put};
use axum::Router;
use sqlx::PgPool;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

// Shared by all handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub notifications_url: String,
    pub http_client: reqwest::Client,
}

#[derive(OpenApi)]
#[openapi(
    info(title = "CinemaGo users service", version = "1.0.0"),
    paths(
        handlers::health,
        handlers::get_users,
        handlers::get_user,
        handlers::register,
        handlers::login,
        handlers::update_user,
        handlers::update_role,
        handlers::delete_user,
        handlers::logout
    ),
    modifiers(&BearerAuth)
)]
struct ApiDoc;

// Adds the "Authorize" button for JWT in Swagger
struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[tokio::main]
async fn main() {
    // Loads .env when running locally, in docker compose sets the variables
    dotenvy::dotenv().ok();

    let port = env::var("PORT").unwrap_or("8080".to_string());
    let database_url = env::var("DB_URL").expect("DB_URL is not set");
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET is not set");
    let notifications_url =
        env::var("NOTIFICATIONS_URL").unwrap_or("http://notifications:8080".to_string());

    let pool = db::connect(&database_url).await;
    db::create_tables(&pool).await;

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("could not create http client");

    let state = AppState {
        pool,
        jwt_secret,
        notifications_url,
        http_client,
    };

    let app = Router::new()
        .route("/health", get(handlers::health))
        .route("/users", get(handlers::get_users))
        .route("/users/register", post(handlers::register))
        .route("/users/login", post(handlers::login))
        .route(
            "/users/{id}",
            get(handlers::get_user)
                .put(handlers::update_user)
                .delete(handlers::delete_user),
        )
        .route("/users/{id}/role", put(handlers::update_role))
        .route("/users/{id}/sessions", delete(handlers::logout))
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .with_state(state);

    let address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    println!("users service running on {address}");
    axum::serve(listener, app).await.unwrap();
}
