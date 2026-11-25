use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::errors::ApiError;
use crate::handlers::{auth_handlers, health, user_handlers};
use crate::openapi::ApiDoc;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/users", get(user_handlers::get_users))
        .route("/users/register", post(auth_handlers::register))
        .route("/users/login", post(auth_handlers::login))
        .route("/users/refresh", post(auth_handlers::refresh))
        .route(
            "/users/{id}",
            get(user_handlers::get_user)
                .put(user_handlers::update_user)
                .delete(user_handlers::delete_user),
        )
        .route("/users/{id}/role", put(user_handlers::update_role))
        .route("/users/{id}/sessions", delete(auth_handlers::logout))
        .route("/users/me/password", put(auth_handlers::change_password))
        .route("/auth/verify", get(auth_handlers::verify))
        // Relative spec url so Swagger also works behind Traefik on /api/users/docs/
        .merge(
            SwaggerUi::new("/docs")
                .url("/openapi.json", ApiDoc::openapi())
                .config(Config::new(["../openapi.json"])),
        )
        // Unknown paths also get a JSON error
        .fallback(not_found)
        // One log line per request with method, path, status and time
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state)
}

async fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "not found")
}
