// Business rules for reservations.
// Handlers call these functions, these functions call the repository (db/).

use axum::http::StatusCode;
use validator::Validate;

use crate::auth::current_user::CurrentUser;
use crate::clients::screenings;
use crate::db::reservation_repository;
use crate::errors::ApiError;
use crate::models::requests::{CreateReservationRequest, UpdateReservationRequest};
use crate::models::reservation::ReservationRow;
use crate::models::responses::{ReservationPage, ReservationResponse, TakenSeatsResponse};
use crate::state::AppState;

const DEFAULT_PAGE_SIZE: i64 = 20;
const MAX_PAGE_SIZE: i64 = 100;

pub async fn list_reservations(
    state: &AppState,
    current_user: &CurrentUser,
    page: Option<i64>,
    page_size: Option<i64>,
    requested_user_id: Option<i32>,
) -> Result<ReservationPage, ApiError> {
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    if page < 1 || !(1..=MAX_PAGE_SIZE).contains(&page_size) {
        return Err(ApiError::bad_request(
            "page must be at least 1 and pageSize 1 to 100",
        ));
    }
    let user_filter = user_filter_for(current_user, requested_user_id)?;

    let offset = (page - 1) * page_size;
    let reservations =
        reservation_repository::get_reservations_page(&state.pool, user_filter, page_size, offset)
            .await?;
    let total = reservation_repository::count_reservations(&state.pool, user_filter).await?;

    let mut items = Vec::new();
    for reservation in reservations {
        items.push(with_seats(state, &reservation).await?);
    }
    Ok(ReservationPage {
        items,
        page,
        page_size,
        total,
    })
}

pub async fn get_reservation(
    state: &AppState,
    current_user: &CurrentUser,
    reservation_id: i32,
) -> Result<ReservationResponse, ApiError> {
    let reservation = find_reservation_for_user(state, current_user, reservation_id).await?;
    with_seats(state, &reservation).await
}

pub async fn get_taken_seats(
    state: &AppState,
    screening_id: i32,
) -> Result<TakenSeatsResponse, ApiError> {
    let seats = reservation_repository::get_taken_seats(&state.pool, screening_id).await?;
    Ok(TakenSeatsResponse {
        screening_id,
        seats,
    })
}

pub async fn create_reservation(
    state: &AppState,
    current_user: &CurrentUser,
    request: CreateReservationRequest,
) -> Result<ReservationResponse, ApiError> {
    request.validate()?;
    screenings::check_screening_exists(state, request.screening_id).await?;

    // The reservation and all seats are saved together or not at all
    let mut transaction = state.pool.begin().await?;
    let reservation = reservation_repository::insert_reservation(
        &mut transaction,
        current_user.id,
        request.screening_id,
    )
    .await?;
    reservation_repository::insert_seats(
        &mut transaction,
        reservation.id,
        reservation.screening_id,
        &request.seats,
    )
    .await?;
    transaction.commit().await?;

    with_seats(state, &reservation).await
}

pub async fn change_seats(
    state: &AppState,
    current_user: &CurrentUser,
    reservation_id: i32,
    request: UpdateReservationRequest,
) -> Result<ReservationResponse, ApiError> {
    request.validate()?;
    find_reservation_for_user(state, current_user, reservation_id).await?;

    let mut transaction = state.pool.begin().await?;
    let reservation =
        reservation_repository::touch_pending_reservation(&mut transaction, reservation_id)
            .await?
            .ok_or_else(not_pending_error)?;
    reservation_repository::delete_all_seats(&mut transaction, reservation.id).await?;
    reservation_repository::insert_seats(
        &mut transaction,
        reservation.id,
        reservation.screening_id,
        &request.seats,
    )
    .await?;
    transaction.commit().await?;

    with_seats(state, &reservation).await
}

pub async fn cancel_reservation(
    state: &AppState,
    current_user: &CurrentUser,
    reservation_id: i32,
) -> Result<(), ApiError> {
    find_reservation_for_user(state, current_user, reservation_id).await?;

    let mut transaction = state.pool.begin().await?;
    reservation_repository::mark_cancelled(&mut transaction, reservation_id)
        .await?
        .ok_or_else(|| ApiError::conflict("reservation is already cancelled"))?;
    reservation_repository::free_seats(&mut transaction, reservation_id).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn remove_seat(
    state: &AppState,
    current_user: &CurrentUser,
    reservation_id: i32,
    seat_id: i32,
) -> Result<(), ApiError> {
    find_reservation_for_user(state, current_user, reservation_id).await?;

    // The seats are counted inside the transaction, after the reservation row is locked
    let mut transaction = state.pool.begin().await?;
    reservation_repository::touch_pending_reservation(&mut transaction, reservation_id)
        .await?
        .ok_or_else(not_pending_error)?;
    let seats = reservation_repository::get_seats(&mut *transaction, reservation_id).await?;
    if !seats.iter().any(|seat| seat.id == seat_id) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "seat not found"));
    }
    if seats.len() == 1 {
        return Err(ApiError::conflict(
            "cannot remove the last seat, cancel the reservation instead",
        ));
    }
    reservation_repository::delete_seat(&mut transaction, reservation_id, seat_id).await?;
    transaction.commit().await?;
    Ok(())
}

// Loads the reservation and checks that the user may see or change it
async fn find_reservation_for_user(
    state: &AppState,
    current_user: &CurrentUser,
    reservation_id: i32,
) -> Result<ReservationRow, ApiError> {
    let reservation = reservation_repository::get_reservation(&state.pool, reservation_id)
        .await?
        .ok_or(ApiError::not_found())?;
    check_owner_or_admin(current_user, &reservation)?;
    Ok(reservation)
}

fn check_owner_or_admin(
    current_user: &CurrentUser,
    reservation: &ReservationRow,
) -> Result<(), ApiError> {
    if current_user.id == reservation.user_id || current_user.is_admin() {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}

// Admins can filter by any user, normal users only see their own reservations
fn user_filter_for(
    current_user: &CurrentUser,
    requested_user_id: Option<i32>,
) -> Result<Option<i32>, ApiError> {
    if current_user.is_admin() {
        return Ok(requested_user_id);
    }
    match requested_user_id {
        Some(other_user_id) if other_user_id != current_user.id => Err(ApiError::forbidden()),
        _ => Ok(Some(current_user.id)),
    }
}

fn not_pending_error() -> ApiError {
    ApiError::conflict("only pending reservations can be changed")
}

async fn with_seats(
    state: &AppState,
    reservation: &ReservationRow,
) -> Result<ReservationResponse, ApiError> {
    let seats = reservation_repository::get_seats(&state.pool, reservation.id).await?;
    Ok(reservation.to_response(seats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::requests::SeatRequest;
    use crate::models::reservation::STATUS_PENDING;

    fn user(id: i32, role: &str) -> CurrentUser {
        CurrentUser {
            id,
            role: role.to_string(),
        }
    }

    fn reservation_of(user_id: i32) -> ReservationRow {
        ReservationRow {
            id: 1,
            user_id,
            screening_id: 3,
            status: STATUS_PENDING.to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn create_request(seats: Vec<(i32, i32)>) -> CreateReservationRequest {
        CreateReservationRequest {
            screening_id: 3,
            seats: seats
                .into_iter()
                .map(|(row, number)| SeatRequest { row, number })
                .collect(),
        }
    }

    fn validation_message(request: &CreateReservationRequest) -> String {
        ApiError::from(request.validate().unwrap_err()).message
    }

    #[test]
    fn owner_and_admin_can_access_others_cannot() {
        let reservation = reservation_of(7);
        assert!(check_owner_or_admin(&user(7, "user"), &reservation).is_ok());
        assert!(check_owner_or_admin(&user(1, "admin"), &reservation).is_ok());
        let error = check_owner_or_admin(&user(8, "user"), &reservation).unwrap_err();
        assert_eq!(error.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn normal_user_only_sees_own_reservations() {
        let normal_user = user(7, "user");
        assert_eq!(user_filter_for(&normal_user, None).unwrap(), Some(7));
        assert_eq!(user_filter_for(&normal_user, Some(7)).unwrap(), Some(7));
        assert!(user_filter_for(&normal_user, Some(8)).is_err());
    }

    #[test]
    fn admin_can_filter_by_any_user() {
        let admin = user(1, "admin");
        assert_eq!(user_filter_for(&admin, None).unwrap(), None);
        assert_eq!(user_filter_for(&admin, Some(8)).unwrap(), Some(8));
    }

    #[test]
    fn seat_rules_come_from_one_validate_call() {
        assert!(create_request(vec![(5, 12), (5, 13)]).validate().is_ok());
        assert_eq!(
            validation_message(&create_request(vec![])),
            "seats: must have 1 to 10 seats"
        );
        assert_eq!(
            validation_message(&create_request(vec![(0, 1)])),
            "seats: row and number must be 1 to 50"
        );
        assert_eq!(
            validation_message(&create_request(vec![(2, 3), (2, 3)])),
            "seats: the same seat is listed twice"
        );
    }
}
