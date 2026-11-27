use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::handlers::{health, reservation_handlers};

const API_DESCRIPTION: &str = "Reservations service of CinemaGo. Users book seats for a screening, change or remove seats while the reservation is pending, and cancel it.

## How to call it

- **Through the Traefik gateway** (server `/api`, for example `http://localhost:8080/api/reservations`): log in with `POST /api/users/login`, then send `Authorization: Bearer <token>` (use the Authorize button). Traefik checks the token with the users service and passes `X-User-Id` and `X-User-Role` on to this service. Headers with those names sent by the client are replaced by Traefik.
- **Directly** (server `/`, dev port 8084): there is no token check, set `X-User-Id` and `X-User-Role` yourself.

Without a logged in user the service answers 401. Users can only see and change their own reservations (403 otherwise), admins can see and change all of them.

Public without a token: `GET /health`, `GET /reservations/screenings/{screeningId}/seats`, `/docs` and `/openapi.json`.

## Reservation status

`pending` after booking, `paid` after payment (payments service, later), `cancelled` after cancel. Seats can only be changed while the reservation is `pending`.

## Seats

A seat is a row and a seat number, both 1 to 50. One reservation has 1 to 10 seats. A seat can only be taken once per screening, cancelled reservations and removed seats free their seats again.

## Errors

Every error from this service is JSON like `{\"error\": \"reservation not found\"}`. This includes bad JSON bodies (400, 415, 422), bad path or query values (400) and unknown paths (404). When the service is down, the gateway itself answers 502 or 503 without a JSON body.";

#[derive(OpenApi)]
#[openapi(
    info(
        title = "CinemaGo reservations service",
        version = "1.0.0",
        description = API_DESCRIPTION
    ),
    servers(
        (url = "/", description = "Direct to the reservations service (dev port 8084), set X-User-Id and X-User-Role yourself"),
        (url = "/api", description = "Through the Traefik gateway (port 8080), use a bearer token")
    ),
    tags(
        (name = "reservations", description = "Book, list, change and cancel seat reservations"),
        (name = "health", description = "Service status")
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
        // No license in Cargo.toml, so leave it out instead of an empty one
        openapi.info.license = None;
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some(
                            "Access token from POST /api/users/login (valid 15 minutes). Only checked by Traefik, which then sets X-User-Id and X-User-Role.",
                        ))
                        .build(),
                ),
            );
        }
    }
}
