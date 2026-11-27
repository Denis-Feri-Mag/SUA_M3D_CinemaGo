use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::{Validate, ValidationError};

// The validate rules are checked in the handlers with request.validate(),
// the schema rules only show the same limits in Swagger

/// One seat in the hall, picked by row and seat number.
#[derive(Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeatRequest {
    /// Row number, counted from the screen, 1 to 50.
    #[schema(minimum = 1, maximum = 50, example = 5)]
    pub row: i32,
    /// Seat number in the row, 1 to 50.
    #[schema(minimum = 1, maximum = 50, example = 12)]
    pub number: i32,
}

/// Body for booking seats. The same seat cannot be listed twice.
#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({ "screeningId": 3, "seats": [{ "row": 5, "number": 12 }, { "row": 5, "number": 13 }] }))]
pub struct CreateReservationRequest {
    /// Id of the screening from the screenings service, at least 1.
    #[validate(range(min = 1, message = "must be at least 1"))]
    #[schema(minimum = 1, example = 3)]
    pub screening_id: i32,
    /// Seats to book, 1 to 10 different seats.
    #[validate(
        length(min = 1, max = 10, message = "must have 1 to 10 seats"),
        custom(function = "check_seats")
    )]
    #[schema(min_items = 1, max_items = 10)]
    pub seats: Vec<SeatRequest>,
}

/// Body for changing seats. The new list replaces all old seats of the reservation.
#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schema(example = json!({ "seats": [{ "row": 6, "number": 7 }, { "row": 6, "number": 8 }] }))]
pub struct UpdateReservationRequest {
    /// New seats, 1 to 10 different seats on the same screening.
    #[validate(
        length(min = 1, max = 10, message = "must have 1 to 10 seats"),
        custom(function = "check_seats")
    )]
    #[schema(min_items = 1, max_items = 10)]
    pub seats: Vec<SeatRequest>,
}

// Custom validator rule for the seats list:
// row and number must be 1 to 50 and the same seat cannot be in the list twice
fn check_seats(seats: &[SeatRequest]) -> Result<(), ValidationError> {
    let mut seen_seats = HashSet::new();
    for seat in seats {
        if !(1..=50).contains(&seat.row) || !(1..=50).contains(&seat.number) {
            return Err(seat_error("row and number must be 1 to 50"));
        }
        if !seen_seats.insert((seat.row, seat.number)) {
            return Err(seat_error("the same seat is listed twice"));
        }
    }
    Ok(())
}

fn seat_error(message: &'static str) -> ValidationError {
    ValidationError::new("seats").with_message(message.into())
}

// Only for Swagger: Traefik sets these headers after the token check,
// when calling the service directly (dev port) you can set them yourself
#[derive(IntoParams)]
#[into_params(parameter_in = Header)]
#[allow(dead_code)]
pub struct UserHeaders {
    /// Id of the logged in user. Set by Traefik from the token, only send it yourself when calling the service directly.
    #[param(rename = "X-User-Id", example = 7)]
    pub user_id: Option<i32>,
    /// Role of the logged in user, `user` or `admin`. Set by Traefik like X-User-Id.
    #[param(rename = "X-User-Role", example = "user")]
    pub user_role: Option<String>,
}

// ?page=1&pageSize=20&userId=3 on GET /reservations
#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ReservationListQuery {
    /// Page number, starts at 1. Default 1.
    #[param(minimum = 1, example = 1)]
    pub page: Option<i64>,
    /// Reservations per page, 1 to 100. Default 20.
    #[param(minimum = 1, maximum = 100, example = 20)]
    pub page_size: Option<i64>,
    /// Only reservations of this user. Admins can use any id, a normal user only their own id (other ids give 403).
    #[param(example = 7)]
    pub user_id: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(row: i32, number: i32) -> SeatRequest {
        SeatRequest { row, number }
    }

    #[test]
    fn good_seats_pass() {
        assert!(check_seats(&[seat(1, 1), seat(1, 2), seat(50, 50)]).is_ok());
    }

    #[test]
    fn seat_out_of_range_fails() {
        assert!(check_seats(&[seat(0, 1)]).is_err());
        assert!(check_seats(&[seat(1, 51)]).is_err());
    }

    #[test]
    fn same_seat_twice_fails() {
        assert!(check_seats(&[seat(2, 3), seat(2, 3)]).is_err());
    }
}
