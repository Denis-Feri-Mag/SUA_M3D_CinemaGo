use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::handlers::{auth_handlers, health, user_handlers};

#[derive(OpenApi)]
#[openapi(
    info(title = "CinemaGo users service", version = "1.0.0"),
    paths(
        health::health,
        user_handlers::get_users,
        user_handlers::get_user,
        auth_handlers::register,
        auth_handlers::login,
        user_handlers::update_user,
        user_handlers::update_role,
        user_handlers::delete_user,
        auth_handlers::logout
    ),
    modifiers(&BearerAuth)
)]
pub struct ApiDoc;

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
