use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::handlers::{health, reservation_handlers};

#[derive(OpenApi)]
#[openapi(
    info(title = "CinemaGo reservations service", version = "1.0.0"),
    servers(
        (url = "/", description = "Direct to the reservations service"),
        (url = "/api", description = "Through the Traefik gateway")
    ),
    paths(
        health::health,
        reservation_handlers::get_reservations,
        reservation_handlers::get_reservation,
        reservation_handlers::get_taken_seats,
        reservation_handlers::create_reservation,
        reservation_handlers::update_reservation,
        reservation_handlers::cancel_reservation,
        reservation_handlers::delete_seat
    ),
    modifiers(&BearerAuth)
)]
pub struct ApiDoc;

// Adds the "Authorize" button for JWT in Swagger (used through Traefik)
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
