use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::models::responses::{ReservationResponse, SeatResponse};

// Values of reservations.status ("paid" comes with the payments service)
pub const STATUS_PENDING: &str = "pending";
pub const STATUS_CANCELLED: &str = "cancelled";

// One row in the reservations table
#[derive(FromRow)]
pub struct ReservationRow {
    pub id: i32,
    pub user_id: i32,
    pub screening_id: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// One row in the reservation_seats table
#[derive(FromRow)]
pub struct SeatRow {
    pub id: i32,
    pub seat_row: i32,
    pub seat_number: i32,
}

impl ReservationRow {
    pub fn to_response(&self, seats: Vec<SeatRow>) -> ReservationResponse {
        let mut seat_list = Vec::new();
        for seat in seats {
            seat_list.push(SeatResponse {
                id: seat.id,
                row: seat.seat_row,
                number: seat.seat_number,
            });
        }
        ReservationResponse {
            id: self.id,
            user_id: self.user_id,
            screening_id: self.screening_id,
            status: self.status.clone(),
            seats: seat_list,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
