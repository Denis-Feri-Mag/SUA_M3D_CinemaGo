// Controllers only read the request (path, query, body, logged in user),
// call the service and turn the result into an HTTP response.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::StatusCode;

use crate::auth::current_user::CurrentUser;
use crate::errors::ApiError;
use crate::extractors::{Json, Path};
use crate::models::requests::{
    CreateReservationRequest, ReservationListQuery, UpdateReservationRequest, UserHeaders,
};
use crate::models::responses::{
    ErrorResponse, ReservationPage, ReservationResponse, TakenSeatsResponse,
};
use crate::services::reservation_service;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/reservations",
    tag = "reservations",
    summary = "List reservations",
    description = "Returns one page of reservations with their seats, sorted by id.\n\n\
**Who can call it:** any logged in user.\n\n\
- A normal user only sees their own reservations. `userId` can be left out or set to their own id, any other id returns 403.\n\
- An admin sees the reservations of all users, or of one user with `?userId=`.\n\n\
Cancelled and paid reservations are included, check `status`.",
    security(("bearerAuth" = [])),
    params(UserHeaders, ReservationListQuery),
    responses(
        (status = 200, description = "One page of reservations.", body = ReservationPage),
        (status = 400, description = "`page` or `pageSize` is out of range or not a number.", body = ErrorResponse, examples(
            ("Out of range" = (summary = "page below 1 or pageSize not 1 to 100", value = json!({ "error": "page must be at least 1 and pageSize 1 to 100" }))),
            ("Not a number" = (summary = "page is not a number", value = json!({ "error": "Failed to deserialize query string: page: invalid digit found in string" })))
        )),
        (status = 401, description = "No logged in user: the X-User-Id or X-User-Role header is missing or invalid. Through the gateway this means the bearer token is missing, expired or invalid (that 401 comes from the users service).", body = ErrorResponse, example = json!({ "error": "missing or invalid user" })),
        (status = 403, description = "A normal user asked for the reservations of another user with `userId`.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn get_reservations(
    State(state): State<AppState>,
    current_user: CurrentUser,
    query: Result<Query<ReservationListQuery>, QueryRejection>,
) -> Result<Json<ReservationPage>, ApiError> {
    let Query(list_query) = query?;
    let page = reservation_service::list_reservations(
        &state,
        &current_user,
        list_query.page,
        list_query.page_size,
        list_query.user_id,
    )
    .await?;
    Ok(Json(page))
}

#[utoipa::path(
    get,
    path = "/reservations/{id}",
    tag = "reservations",
    summary = "Get one reservation",
    description = "Returns one reservation with all its seats and its status.\n\n\
**Who can call it:** the user who made the reservation, or an admin.",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "Reservation id", example = 12), UserHeaders),
    responses(
        (status = 200, description = "The reservation.", body = ReservationResponse),
        (status = 400, description = "The id in the path is not a whole number.", body = ErrorResponse, example = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })),
        (status = 401, description = "No logged in user: the X-User-Id or X-User-Role header is missing or invalid. Through the gateway this means the bearer token is missing, expired or invalid (that 401 comes from the users service).", body = ErrorResponse, example = json!({ "error": "missing or invalid user" })),
        (status = 403, description = "The reservation belongs to another user and you are not an admin.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 404, description = "No reservation with this id.", body = ErrorResponse, example = json!({ "error": "reservation not found" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn get_reservation(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(reservation_id): Path<i32>,
) -> Result<Json<ReservationResponse>, ApiError> {
    let reservation =
        reservation_service::get_reservation(&state, &current_user, reservation_id).await?;
    Ok(Json(reservation))
}

#[utoipa::path(
    get,
    path = "/reservations/screenings/{screeningId}/seats",
    tag = "reservations",
    summary = "Taken seats of a screening",
    description = "Lists the seats of one screening that are already booked (pending or paid reservations). \
The frontend uses it to draw the seat map. Seats of cancelled reservations and removed seats are free again and not listed.\n\n\
**Who can call it:** everyone, no token needed. Through the gateway it is the only public reservations path besides health and docs.\n\n\
The screening is not checked here, an unknown screening simply has no taken seats.",
    params(("screeningId" = i32, Path, description = "Screening id from the screenings service", example = 3)),
    responses(
        (status = 200, description = "Taken seats, sorted by row and number.", body = TakenSeatsResponse, example = json!({ "screeningId": 3, "seats": [{ "row": 5, "number": 12 }, { "row": 5, "number": 13 }] })),
        (status = 400, description = "The screening id in the path is not a whole number.", body = ErrorResponse, example = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn get_taken_seats(
    State(state): State<AppState>,
    Path(screening_id): Path<i32>,
) -> Result<Json<TakenSeatsResponse>, ApiError> {
    let taken_seats = reservation_service::get_taken_seats(&state, screening_id).await?;
    Ok(Json(taken_seats))
}

#[utoipa::path(
    post,
    path = "/reservations",
    tag = "reservations",
    summary = "Book seats",
    description = "Creates a reservation for the logged in user with status `pending`.\n\n\
**Who can call it:** any logged in user. The reservation always belongs to the caller.\n\n\
**Rules:**\n\
- 1 to 10 seats, row and number 1 to 50, the same seat only once in the list.\n\
- The service asks the screenings service (`GET /screenings/{id}`) if the screening exists. If screenings answers 404, the booking fails with 404. If screenings is not reachable, the check is skipped and the booking still works.\n\
- All seats are saved together. If one seat is already taken by a pending or paid reservation, nothing is saved and the answer is 409.",
    security(("bearerAuth" = [])),
    params(UserHeaders),
    request_body(content = CreateReservationRequest, description = "Screening and the seats to book."),
    responses(
        (status = 201, description = "Reservation created with status pending.", body = ReservationResponse, example = json!({
            "id": 12, "userId": 7, "screeningId": 3, "status": "pending",
            "seats": [{ "id": 31, "row": 5, "number": 12 }, { "id": 32, "row": 5, "number": 13 }],
            "createdAt": "2026-12-01T18:30:00Z", "updatedAt": "2026-12-01T18:30:00Z"
        })),
        (status = 400, description = "The body breaks one of the rules.", body = ErrorResponse, examples(
            ("Bad screening id" = (summary = "screeningId is below 1", value = json!({ "error": "screeningId: must be at least 1" }))),
            ("Missing seats" = (summary = "Empty seat list or more than 10 seats", value = json!({ "error": "seats: must have 1 to 10 seats" }))),
                ("Seat out of range" = (summary = "Row or number is not 1 to 50", value = json!({ "error": "seats: row and number must be 1 to 50" }))),
                ("Same seat twice" = (summary = "A seat is listed more than once", value = json!({ "error": "seats: the same seat is listed twice" }))),
                ("Bad JSON" = (summary = "The body is not valid JSON", value = json!({ "error": "Failed to parse the request body as JSON: key must be a string at line 1 column 2" })))
        )),
        (status = 401, description = "No logged in user: the X-User-Id or X-User-Role header is missing or invalid. Through the gateway this means the bearer token is missing, expired or invalid (that 401 comes from the users service).", body = ErrorResponse, example = json!({ "error": "missing or invalid user" })),
        (status = 404, description = "The screenings service says this screening does not exist.", body = ErrorResponse, example = json!({ "error": "screening not found" })),
        (status = 409, description = "At least one seat is already taken on this screening.", body = ErrorResponse, example = json!({ "error": "seat already taken" })),
        (status = 415, description = "The body is not sent as JSON (missing `Content-Type: application/json`).", body = ErrorResponse, example = json!({ "error": "Expected request with `Content-Type: application/json`" })),
        (status = 422, description = "The JSON is valid but a field is missing or has the wrong type.", body = ErrorResponse, example = json!({ "error": "Failed to deserialize the JSON body into the target type: missing field `seats` at line 1 column 17" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn create_reservation(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(request): Json<CreateReservationRequest>,
) -> Result<(StatusCode, Json<ReservationResponse>), ApiError> {
    let reservation =
        reservation_service::create_reservation(&state, &current_user, request).await?;
    Ok((StatusCode::CREATED, Json(reservation)))
}

#[utoipa::path(
    put,
    path = "/reservations/{id}",
    tag = "reservations",
    summary = "Change the seats of a reservation",
    description = "Replaces all seats of a reservation with the seats in the body. The screening stays the same.\n\n\
**Who can call it:** the user who made the reservation, or an admin.\n\n\
**Rules:**\n\
- Only `pending` reservations can be changed, paid or cancelled ones return 409.\n\
- Same seat rules as booking: 1 to 10 seats, row and number 1 to 50, no seat twice.\n\
- Old seats are freed and new seats are taken in one step. Keeping some of the old seats in the list is fine. If a new seat is taken by someone else, nothing changes and the answer is 409.\n\
- Seats get new ids.",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "Reservation id", example = 12), UserHeaders),
    request_body(content = UpdateReservationRequest, description = "The new list of seats."),
    responses(
        (status = 200, description = "Seats changed, returns the reservation with the new seats.", body = ReservationResponse, example = json!({
            "id": 12, "userId": 7, "screeningId": 3, "status": "pending",
            "seats": [{ "id": 35, "row": 6, "number": 7 }, { "id": 36, "row": 6, "number": 8 }],
            "createdAt": "2026-12-01T18:30:00Z", "updatedAt": "2026-12-01T18:35:00Z"
        })),
        (status = 400, description = "Bad id in the path, or the body breaks one of the seat rules.", body = ErrorResponse, examples(
            ("Missing seats" = (summary = "Empty seat list or more than 10 seats", value = json!({ "error": "seats: must have 1 to 10 seats" }))),
                ("Seat out of range" = (summary = "Row or number is not 1 to 50", value = json!({ "error": "seats: row and number must be 1 to 50" }))),
                ("Same seat twice" = (summary = "A seat is listed more than once", value = json!({ "error": "seats: the same seat is listed twice" }))),
                ("Bad JSON" = (summary = "The body is not valid JSON", value = json!({ "error": "Failed to parse the request body as JSON: key must be a string at line 1 column 2" }))),
            ("Bad id" = (summary = "The id in the path is not a number", value = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })))
        )),
        (status = 401, description = "No logged in user: the X-User-Id or X-User-Role header is missing or invalid. Through the gateway this means the bearer token is missing, expired or invalid (that 401 comes from the users service).", body = ErrorResponse, example = json!({ "error": "missing or invalid user" })),
        (status = 403, description = "The reservation belongs to another user and you are not an admin.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 404, description = "No reservation with this id.", body = ErrorResponse, example = json!({ "error": "reservation not found" })),
        (status = 409, description = "The reservation is not pending anymore, or a new seat is already taken.", body = ErrorResponse, examples(
            ("Not pending" = (summary = "Reservation is paid or cancelled", value = json!({ "error": "only pending reservations can be changed" }))),
            ("Seat taken" = (summary = "A new seat is booked by someone else", value = json!({ "error": "seat already taken" })))
        )),
        (status = 415, description = "The body is not sent as JSON (missing `Content-Type: application/json`).", body = ErrorResponse, example = json!({ "error": "Expected request with `Content-Type: application/json`" })),
        (status = 422, description = "The JSON is valid but a field is missing or has the wrong type.", body = ErrorResponse, example = json!({ "error": "Failed to deserialize the JSON body into the target type: missing field `seats` at line 1 column 17" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn update_reservation(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(reservation_id): Path<i32>,
    Json(request): Json<UpdateReservationRequest>,
) -> Result<Json<ReservationResponse>, ApiError> {
    let reservation =
        reservation_service::change_seats(&state, &current_user, reservation_id, request).await?;
    Ok(Json(reservation))
}

#[utoipa::path(
    delete,
    path = "/reservations/{id}",
    tag = "reservations",
    summary = "Cancel a reservation",
    description = "Sets the status to `cancelled` and frees all its seats, so others can book them. \
The reservation itself is kept and still shows up in the list with its old seats.\n\n\
**Who can call it:** the user who made the reservation, or an admin.\n\n\
Pending and paid reservations can be cancelled (refunds for paid ones come with the payments service). Cancelling twice returns 409.",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "Reservation id", example = 12), UserHeaders),
    responses(
        (status = 204, description = "Cancelled, the seats are free again. No body."),
        (status = 400, description = "The id in the path is not a whole number.", body = ErrorResponse, example = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })),
        (status = 401, description = "No logged in user: the X-User-Id or X-User-Role header is missing or invalid. Through the gateway this means the bearer token is missing, expired or invalid (that 401 comes from the users service).", body = ErrorResponse, example = json!({ "error": "missing or invalid user" })),
        (status = 403, description = "The reservation belongs to another user and you are not an admin.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 404, description = "No reservation with this id.", body = ErrorResponse, example = json!({ "error": "reservation not found" })),
        (status = 409, description = "The reservation is already cancelled.", body = ErrorResponse, example = json!({ "error": "reservation is already cancelled" })),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn cancel_reservation(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(reservation_id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    reservation_service::cancel_reservation(&state, &current_user, reservation_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/reservations/{id}/seats/{seatId}",
    tag = "reservations",
    summary = "Remove one seat from a reservation",
    description = "Deletes one seat from a reservation, the seat is free again right away. The seat id is the `id` in the `seats` list of the reservation.\n\n\
**Who can call it:** the user who made the reservation, or an admin.\n\n\
**Rules:**\n\
- Only `pending` reservations can be changed.\n\
- The last seat cannot be removed, cancel the reservation instead.",
    security(("bearerAuth" = [])),
    params(
        ("id" = i32, Path, description = "Reservation id", example = 12),
        ("seatId" = i32, Path, description = "Seat id from the seats list of this reservation", example = 32),
        UserHeaders
    ),
    responses(
        (status = 204, description = "Seat removed. No body."),
        (status = 400, description = "An id in the path is not a whole number.", body = ErrorResponse, example = json!({ "error": "Invalid URL: Cannot parse `abc` to a `i32`" })),
        (status = 401, description = "No logged in user: the X-User-Id or X-User-Role header is missing or invalid. Through the gateway this means the bearer token is missing, expired or invalid (that 401 comes from the users service).", body = ErrorResponse, example = json!({ "error": "missing or invalid user" })),
        (status = 403, description = "The reservation belongs to another user and you are not an admin.", body = ErrorResponse, example = json!({ "error": "not allowed" })),
        (status = 404, description = "No reservation with this id, or the seat is not part of this reservation.", body = ErrorResponse, examples(
            ("Reservation" = (summary = "Unknown reservation", value = json!({ "error": "reservation not found" }))),
            ("Seat" = (summary = "Seat id is not in this reservation", value = json!({ "error": "seat not found" })))
        )),
        (status = 409, description = "The reservation is not pending anymore, or this is its last seat.", body = ErrorResponse, examples(
            ("Not pending" = (summary = "Reservation is paid or cancelled", value = json!({ "error": "only pending reservations can be changed" }))),
            ("Last seat" = (summary = "Only one seat left", value = json!({ "error": "cannot remove the last seat, cancel the reservation instead" })))
        )),
        (status = 500, description = "Unexpected database error. The details are only in the service log.", body = ErrorResponse, example = json!({ "error": "database error" }))
    )
)]
pub async fn delete_seat(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path((reservation_id, seat_id)): Path<(i32, i32)>,
) -> Result<StatusCode, ApiError> {
    reservation_service::remove_seat(&state, &current_user, reservation_id, seat_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
