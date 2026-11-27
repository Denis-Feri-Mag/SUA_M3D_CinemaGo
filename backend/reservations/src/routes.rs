use axum::http::StatusCode;
use axum::routing::{delete, get};
use axum::Router;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::errors::ApiError;
use crate::handlers::{health, reservation_handlers};
use crate::openapi::ApiDoc;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route(
            "/reservations",
            get(reservation_handlers::get_reservations)
                .post(reservation_handlers::create_reservation),
        )
        .route(
            "/reservations/{id}",
            get(reservation_handlers::get_reservation)
                .put(reservation_handlers::update_reservation)
                .delete(reservation_handlers::cancel_reservation),
        )
        .route(
            "/reservations/{id}/seats/{seat_id}",
            delete(reservation_handlers::delete_seat),
        )
        .route(
            "/reservations/screenings/{screening_id}/seats",
            get(reservation_handlers::get_taken_seats),
        )
        // Relative spec url so Swagger also works behind Traefik on /api/reservations/docs/
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
