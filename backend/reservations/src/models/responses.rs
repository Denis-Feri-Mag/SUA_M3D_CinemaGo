use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeatResponse {
    pub id: i32,
    pub row: i32,
    pub number: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationResponse {
    pub id: i32,
    pub user_id: i32,
    pub screening_id: i32,
    /// pending, paid or cancelled
    pub status: String,
    pub seats: Vec<SeatResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// One page of GET /reservations
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationPage {
    pub items: Vec<ReservationResponse>,
    pub page: i64,
    pub page_size: i64,
    /// Number of all matching reservations
    pub total: i64,
}

#[derive(Serialize, ToSchema)]
pub struct TakenSeat {
    pub row: i32,
    pub number: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TakenSeatsResponse {
    pub screening_id: i32,
    pub seats: Vec<TakenSeat>,
}

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}
