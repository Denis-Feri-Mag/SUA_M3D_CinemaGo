use axum::Json;

use crate::models::responses::HealthResponse;

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    summary = "Health check",
    description = "Returns `{\"status\":\"ok\"}` when the service is running. No token needed. Docker and Traefik use it to know when the service is ready.",
    responses(
        (status = 200, description = "Service is running", body = HealthResponse, example = json!({ "status": "ok" }))
    )
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}
