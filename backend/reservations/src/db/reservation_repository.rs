// Data layer: only SQL, no business rules.
// Functions that change data take a connection from a transaction
// (the service starts and commits it), simple reads take the pool.

use sqlx::{PgConnection, PgExecutor, PgPool};

use crate::models::requests::SeatRequest;
use crate::models::reservation::{ReservationRow, SeatRow, STATUS_CANCELLED, STATUS_PENDING};
use crate::models::responses::TakenSeat;

// user_id None means all users (admin)
pub async fn get_reservations_page(
    pool: &PgPool,
    user_id: Option<i32>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ReservationRow>, sqlx::Error> {
    sqlx::query_as::<_, ReservationRow>(
        "SELECT * FROM reservations
         WHERE ($1::INTEGER IS NULL OR user_id = $1)
         ORDER BY id LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn count_reservations(pool: &PgPool, user_id: Option<i32>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM reservations WHERE ($1::INTEGER IS NULL OR user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn get_reservation(
    pool: &PgPool,
    reservation_id: i32,
) -> Result<Option<ReservationRow>, sqlx::Error> {
    sqlx::query_as::<_, ReservationRow>("SELECT * FROM reservations WHERE id = $1")
        .bind(reservation_id)
        .fetch_optional(pool)
        .await
}

// Works with the pool and inside a transaction
pub async fn get_seats<'connection>(
    executor: impl PgExecutor<'connection>,
    reservation_id: i32,
) -> Result<Vec<SeatRow>, sqlx::Error> {
    sqlx::query_as::<_, SeatRow>(
        "SELECT id, seat_row, seat_number FROM reservation_seats
         WHERE reservation_id = $1 ORDER BY seat_row, seat_number",
    )
    .bind(reservation_id)
    .fetch_all(executor)
    .await
}

pub async fn get_taken_seats(
    pool: &PgPool,
    screening_id: i32,
) -> Result<Vec<TakenSeat>, sqlx::Error> {
    let rows: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT seat_row, seat_number FROM reservation_seats
         WHERE screening_id = $1 AND cancelled_at IS NULL
         ORDER BY seat_row, seat_number",
    )
    .bind(screening_id)
    .fetch_all(pool)
    .await?;

    let mut taken_seats = Vec::new();
    for (row, number) in rows {
        taken_seats.push(TakenSeat { row, number });
    }
    Ok(taken_seats)
}

pub async fn insert_reservation(
    connection: &mut PgConnection,
    user_id: i32,
    screening_id: i32,
) -> Result<ReservationRow, sqlx::Error> {
    sqlx::query_as::<_, ReservationRow>(
        "INSERT INTO reservations (user_id, screening_id, status) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(user_id)
    .bind(screening_id)
    .bind(STATUS_PENDING)
    .fetch_one(connection)
    .await
}

// A taken seat breaks the unique index, the error becomes 409 in errors.rs
pub async fn insert_seats(
    connection: &mut PgConnection,
    reservation_id: i32,
    screening_id: i32,
    seats: &[SeatRequest],
) -> Result<(), sqlx::Error> {
    for seat in seats {
        sqlx::query(
            "INSERT INTO reservation_seats (reservation_id, screening_id, seat_row, seat_number)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(reservation_id)
        .bind(screening_id)
        .bind(seat.row)
        .bind(seat.number)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

// Only changes a pending reservation. The row stays locked until the transaction ends,
// so two requests cannot change the same reservation at the same time.
// None means the reservation is not pending (paid or cancelled).
pub async fn touch_pending_reservation(
    connection: &mut PgConnection,
    reservation_id: i32,
) -> Result<Option<ReservationRow>, sqlx::Error> {
    sqlx::query_as::<_, ReservationRow>(
        "UPDATE reservations SET updated_at = NOW()
         WHERE id = $1 AND status = $2 RETURNING *",
    )
    .bind(reservation_id)
    .bind(STATUS_PENDING)
    .fetch_optional(connection)
    .await
}

// None means it was already cancelled
pub async fn mark_cancelled(
    connection: &mut PgConnection,
    reservation_id: i32,
) -> Result<Option<ReservationRow>, sqlx::Error> {
    sqlx::query_as::<_, ReservationRow>(
        "UPDATE reservations SET status = $2, updated_at = NOW()
         WHERE id = $1 AND status <> $2 RETURNING *",
    )
    .bind(reservation_id)
    .bind(STATUS_CANCELLED)
    .fetch_optional(connection)
    .await
}

// Cancelled seats stay saved but do not count as taken anymore
pub async fn free_seats(
    connection: &mut PgConnection,
    reservation_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE reservation_seats SET cancelled_at = NOW()
         WHERE reservation_id = $1 AND cancelled_at IS NULL",
    )
    .bind(reservation_id)
    .execute(connection)
    .await?;
    Ok(())
}

pub async fn delete_all_seats(
    connection: &mut PgConnection,
    reservation_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM reservation_seats WHERE reservation_id = $1")
        .bind(reservation_id)
        .execute(connection)
        .await?;
    Ok(())
}

pub async fn delete_seat(
    connection: &mut PgConnection,
    reservation_id: i32,
    seat_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM reservation_seats WHERE id = $1 AND reservation_id = $2")
        .bind(seat_id)
        .bind(reservation_id)
        .execute(connection)
        .await?;
    Ok(())
}
