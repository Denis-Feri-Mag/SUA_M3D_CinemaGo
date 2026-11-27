use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{middleware, Router};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::controllers::{auth_controller, health_controller, user_controller};
use crate::errors::ApiError;
use crate::openapi::ApiDoc;
use crate::rate_limit;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    // Public auth routes get a rate limit against password guessing and spam
    let rate_limited_routes = Router::new()
        .route("/users/register", post(auth_controller::register))
        .route("/users/login", post(auth_controller::login))
        .route("/users/refresh", post(auth_controller::refresh))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit::limit_requests,
        ));

    Router::new()
        .route("/health", get(health_controller::health))
        .route("/users", get(user_controller::get_users))
        .merge(rate_limited_routes)
        .route(
            "/users/me",
            get(user_controller::get_me).put(user_controller::update_me),
        )
        .route(
            "/users/{id}",
            get(user_controller::get_user)
                .put(user_controller::update_user)
                .delete(user_controller::delete_user),
        )
        .route("/users/{id}/role", put(user_controller::update_role))
        .route("/users/{id}/sessions", delete(auth_controller::logout))
        .route("/users/me/password", put(auth_controller::change_password))
        .route("/auth/verify", get(auth_controller::verify))
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
