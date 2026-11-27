use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// A booked seat.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeatResponse {
    /// Seat id, used to remove one seat with DELETE /reservations/{id}/seats/{seatId}.
    #[schema(example = 31)]
    pub id: i32,
    /// Row number, 1 to 50.
    #[schema(example = 5)]
    pub row: i32,
    /// Seat number in the row, 1 to 50.
    #[schema(example = 12)]
    pub number: i32,
}

/// A reservation with its seats.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationResponse {
    /// Reservation id.
    #[schema(example = 12)]
    pub id: i32,
    /// Id of the user who made the reservation (from the users service).
    #[schema(example = 7)]
    pub user_id: i32,
    /// Id of the screening (from the screenings service).
    #[schema(example = 3)]
    pub screening_id: i32,
    /// `pending` after booking, `paid` after payment, `cancelled` after cancel. Only pending reservations can be changed.
    #[schema(example = "pending")]
    pub status: String,
    /// Booked seats, sorted by row and number. Seats of a cancelled reservation are still listed but no longer taken.
    pub seats: Vec<SeatResponse>,
    /// When the reservation was made (UTC).
    #[schema(example = "2026-12-01T18:30:00Z")]
    pub created_at: DateTime<Utc>,
    /// Last change of status or seats (UTC).
    #[schema(example = "2026-12-01T18:35:00Z")]
    pub updated_at: DateTime<Utc>,
}

/// One page of reservations.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReservationPage {
    /// Reservations on this page, sorted by id.
    pub items: Vec<ReservationResponse>,
    /// Page number that was returned.
    #[schema(example = 1)]
    pub page: i64,
    /// Page size that was used.
    #[schema(example = 20)]
    pub page_size: i64,
    /// Number of all matching reservations on all pages.
    #[schema(example = 1)]
    pub total: i64,
}

/// A seat that is already booked by someone.
#[derive(Serialize, ToSchema)]
pub struct TakenSeat {
    /// Row number.
    #[schema(example = 5)]
    pub row: i32,
    /// Seat number in the row.
    #[schema(example = 12)]
    pub number: i32,
}

/// Taken seats of one screening. Seats of cancelled reservations are free and not listed.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TakenSeatsResponse {
    /// The screening id from the path.
    #[schema(example = 3)]
    pub screening_id: i32,
    /// Taken seats, sorted by row and number. Empty when nothing is booked yet.
    pub seats: Vec<TakenSeat>,
}

/// Health check answer.
#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    /// Always `ok` when the service is running.
    #[schema(example = "ok")]
    pub status: String,
}

/// Every error in this service has this shape.
#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Short message that says what went wrong.
    #[schema(example = "reservation not found")]
    pub error: String,
}
