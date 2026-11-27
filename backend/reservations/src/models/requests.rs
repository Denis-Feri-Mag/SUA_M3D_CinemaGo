use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::errors::ApiError;

// The validate rules are checked in the handlers with request.validate(),
// the schema rules only show the same limits in Swagger

#[derive(Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeatRequest {
    #[schema(minimum = 1, maximum = 50, example = 5)]
    pub row: i32,
    #[schema(minimum = 1, maximum = 50, example = 12)]
    pub number: i32,
}

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateReservationRequest {
    #[validate(range(min = 1, message = "must be at least 1"))]
    #[schema(minimum = 1, example = 3)]
    pub screening_id: i32,
    #[validate(length(min = 1, max = 10, message = "must have 1 to 10 seats"))]
    #[schema(min_items = 1, max_items = 10)]
    pub seats: Vec<SeatRequest>,
}

#[derive(Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReservationRequest {
    #[validate(length(min = 1, max = 10, message = "must have 1 to 10 seats"))]
    #[schema(min_items = 1, max_items = 10)]
    pub seats: Vec<SeatRequest>,
}

// Row and number must be 1 to 50 and the same seat cannot be in the list twice
pub fn check_seats(seats: &[SeatRequest]) -> Result<(), ApiError> {
    let mut seen_seats = HashSet::new();
    for seat in seats {
        if !(1..=50).contains(&seat.row) || !(1..=50).contains(&seat.number) {
            return Err(ApiError::bad_request(
                "seats: row and number must be 1 to 50",
            ));
        }
        if !seen_seats.insert((seat.row, seat.number)) {
            return Err(ApiError::bad_request(
                "seats: the same seat is listed twice",
            ));
        }
    }
    Ok(())
}

// Only for Swagger: Traefik sets these headers after the token check,
// when calling the service directly (dev port) you can set them yourself
#[derive(IntoParams)]
#[into_params(parameter_in = Header)]
#[allow(dead_code)]
pub struct UserHeaders {
    /// Id of the logged in user
    #[param(rename = "X-User-Id")]
    pub user_id: Option<i32>,
    /// user or admin
    #[param(rename = "X-User-Role")]
    pub user_role: Option<String>,
}

// ?page=1&pageSize=20&userId=3 on GET /reservations
#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ReservationListQuery {
    /// Page number, starts at 1 (default 1)
    #[param(minimum = 1)]
    pub page: Option<i64>,
    /// Reservations per page, 1 to 100 (default 20)
    #[param(minimum = 1, maximum = 100)]
    pub page_size: Option<i64>,
    /// Only for admins: reservations of one user
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
