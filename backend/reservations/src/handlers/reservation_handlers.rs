use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use validator::Validate;

use crate::auth::current_user::{self, CurrentUser};
use crate::clients::screenings;
use crate::db::reservation_queries;
use crate::errors::ApiError;
use crate::extractors::{Json, Path};
use crate::models::requests::{
    check_seats, CreateReservationRequest, ReservationListQuery, UpdateReservationRequest,
    UserHeaders,
};
use crate::models::reservation::ReservationRow;
use crate::models::responses::{
    ErrorResponse, ReservationPage, ReservationResponse, TakenSeatsResponse,
};
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/reservations",
    tag = "reservations",
    security(("bearerAuth" = [])),
    params(UserHeaders, ReservationListQuery),
    responses(
        (status = 200, description = "Your reservations, admins see all", body = ReservationPage),
        (status = 400, description = "Bad page or pageSize", body = ErrorResponse),
        (status = 401, description = "Not logged in", body = ErrorResponse),
        (status = 403, description = "userId of someone else (only admins)", body = ErrorResponse)
    )
)]
pub async fn get_reservations(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<ReservationListQuery>, QueryRejection>,
) -> Result<Json<ReservationPage>, ApiError> {
    let user = current_user::from_headers(&headers)?;
    let Query(list_query) = query?;

    let page = list_query.page.unwrap_or(1);
    let page_size = list_query.page_size.unwrap_or(20);
    if page < 1 || !(1..=100).contains(&page_size) {
        return Err(ApiError::bad_request(
            "page must be at least 1 and pageSize 1 to 100",
        ));
    }

    // Normal users only see their own reservations
    let user_filter = if user.is_admin() {
        list_query.user_id
    } else {
        match list_query.user_id {
            Some(other_user_id) if other_user_id != user.id => return Err(ApiError::forbidden()),
            _ => Some(user.id),
        }
    };

    let offset = (page - 1) * page_size;
    let reservations =
        reservation_queries::get_reservations_page(&state.pool, user_filter, page_size, offset)
            .await?;
    let total = reservation_queries::count_reservations(&state.pool, user_filter).await?;

    let mut items = Vec::new();
    for reservation in reservations {
        let seats = reservation_queries::get_seats(&state.pool, reservation.id).await?;
        items.push(reservation.to_response(seats));
    }
    Ok(Json(ReservationPage {
        items,
        page,
        page_size,
        total,
    }))
}

#[utoipa::path(
    get,
    path = "/reservations/{id}",
    tag = "reservations",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "Reservation id"), UserHeaders),
    responses(
        (status = 200, description = "One reservation with its seats", body = ReservationResponse),
        (status = 401, description = "Not logged in", body = ErrorResponse),
        (status = 403, description = "Not your reservation", body = ErrorResponse),
        (status = 404, description = "Reservation not found", body = ErrorResponse)
    )
)]
pub async fn get_reservation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reservation_id): Path<i32>,
) -> Result<Json<ReservationResponse>, ApiError> {
    let user = current_user::from_headers(&headers)?;
    let reservation = find_own_reservation(&state, &user, reservation_id).await?;
    let seats = reservation_queries::get_seats(&state.pool, reservation.id).await?;
    Ok(Json(reservation.to_response(seats)))
}

#[utoipa::path(
    get,
    path = "/reservations/screenings/{screeningId}/seats",
    tag = "reservations",
    params(("screeningId" = i32, Path, description = "Screening id")),
    responses(
        (status = 200, description = "Seats that are already taken, no token needed", body = TakenSeatsResponse),
        (status = 400, description = "Bad screening id", body = ErrorResponse)
    )
)]
pub async fn get_taken_seats(
    State(state): State<AppState>,
    Path(screening_id): Path<i32>,
) -> Result<Json<TakenSeatsResponse>, ApiError> {
    let seats = reservation_queries::get_taken_seats(&state.pool, screening_id).await?;
    Ok(Json(TakenSeatsResponse {
        screening_id,
        seats,
    }))
}

#[utoipa::path(
    post,
    path = "/reservations",
    tag = "reservations",
    security(("bearerAuth" = [])),
    params(UserHeaders),
    request_body = CreateReservationRequest,
    responses(
        (status = 201, description = "Reservation created as pending", body = ReservationResponse),
        (status = 400, description = "Bad screening id or seats", body = ErrorResponse),
        (status = 401, description = "Not logged in", body = ErrorResponse),
        (status = 404, description = "Screening not found", body = ErrorResponse),
        (status = 409, description = "A seat is already taken", body = ErrorResponse)
    )
)]
pub async fn create_reservation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateReservationRequest>,
) -> Result<(StatusCode, Json<ReservationResponse>), ApiError> {
    let user = current_user::from_headers(&headers)?;
    request.validate()?;
    check_seats(&request.seats)?;

    screenings::check_screening_exists(&state, request.screening_id).await?;

    let reservation = reservation_queries::create_reservation(
        &state.pool,
        user.id,
        request.screening_id,
        &request.seats,
    )
    .await?;
    let seats = reservation_queries::get_seats(&state.pool, reservation.id).await?;
    Ok((StatusCode::CREATED, Json(reservation.to_response(seats))))
}

#[utoipa::path(
    put,
    path = "/reservations/{id}",
    tag = "reservations",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "Reservation id"), UserHeaders),
    request_body = UpdateReservationRequest,
    responses(
        (status = 200, description = "Seats changed", body = ReservationResponse),
        (status = 400, description = "Bad seats", body = ErrorResponse),
        (status = 401, description = "Not logged in", body = ErrorResponse),
        (status = 403, description = "Not your reservation", body = ErrorResponse),
        (status = 404, description = "Reservation not found", body = ErrorResponse),
        (status = 409, description = "Not pending anymore or a seat is already taken", body = ErrorResponse)
    )
)]
pub async fn update_reservation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reservation_id): Path<i32>,
    Json(request): Json<UpdateReservationRequest>,
) -> Result<Json<ReservationResponse>, ApiError> {
    let user = current_user::from_headers(&headers)?;
    request.validate()?;
    check_seats(&request.seats)?;

    let reservation = find_own_reservation(&state, &user, reservation_id).await?;
    check_pending(&reservation)?;

    let updated_reservation =
        reservation_queries::replace_seats(&state.pool, &reservation, &request.seats).await?;
    let seats = reservation_queries::get_seats(&state.pool, reservation.id).await?;
    Ok(Json(updated_reservation.to_response(seats)))
}

#[utoipa::path(
    delete,
    path = "/reservations/{id}",
    tag = "reservations",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "Reservation id"), UserHeaders),
    responses(
        (status = 204, description = "Reservation cancelled, its seats are free again"),
        (status = 401, description = "Not logged in", body = ErrorResponse),
        (status = 403, description = "Not your reservation", body = ErrorResponse),
        (status = 404, description = "Reservation not found", body = ErrorResponse),
        (status = 409, description = "Already cancelled", body = ErrorResponse)
    )
)]
pub async fn cancel_reservation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reservation_id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    let user = current_user::from_headers(&headers)?;
    let reservation = find_own_reservation(&state, &user, reservation_id).await?;
    if reservation.status == "cancelled" {
        return Err(ApiError::conflict("reservation is already cancelled"));
    }

    reservation_queries::cancel_reservation(&state.pool, reservation.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/reservations/{id}/seats/{seatId}",
    tag = "reservations",
    security(("bearerAuth" = [])),
    params(
        ("id" = i32, Path, description = "Reservation id"),
        ("seatId" = i32, Path, description = "Seat id from the reservation"),
        UserHeaders
    ),
    responses(
        (status = 204, description = "Seat removed from the reservation"),
        (status = 401, description = "Not logged in", body = ErrorResponse),
        (status = 403, description = "Not your reservation", body = ErrorResponse),
        (status = 404, description = "Reservation or seat not found", body = ErrorResponse),
        (status = 409, description = "Not pending anymore or the last seat", body = ErrorResponse)
    )
)]
pub async fn delete_seat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((reservation_id, seat_id)): Path<(i32, i32)>,
) -> Result<StatusCode, ApiError> {
    let user = current_user::from_headers(&headers)?;
    let reservation = find_own_reservation(&state, &user, reservation_id).await?;
    check_pending(&reservation)?;

    let seats = reservation_queries::get_seats(&state.pool, reservation.id).await?;
    if !seats.iter().any(|seat| seat.id == seat_id) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "seat not found"));
    }
    if seats.len() == 1 {
        return Err(ApiError::conflict(
            "cannot remove the last seat, cancel the reservation instead",
        ));
    }

    reservation_queries::delete_seat(&state.pool, reservation.id, seat_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// Loads the reservation and checks that the user may see or change it
async fn find_own_reservation(
    state: &AppState,
    user: &CurrentUser,
    reservation_id: i32,
) -> Result<ReservationRow, ApiError> {
    let reservation = reservation_queries::get_reservation(&state.pool, reservation_id)
        .await?
        .ok_or(ApiError::not_found())?;
    current_user::check_owner_or_admin(user, reservation.user_id)?;
    Ok(reservation)
}

// Seats can only change before the reservation is paid or cancelled
fn check_pending(reservation: &ReservationRow) -> Result<(), ApiError> {
    if reservation.status != "pending" {
        return Err(ApiError::conflict(
            "only pending reservations can be changed",
        ));
    }
    Ok(())
}
