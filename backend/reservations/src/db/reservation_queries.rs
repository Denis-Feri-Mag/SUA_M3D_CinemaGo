use sqlx::PgPool;

use crate::models::requests::SeatRequest;
use crate::models::reservation::{ReservationRow, SeatRow};
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

pub async fn get_seats(pool: &PgPool, reservation_id: i32) -> Result<Vec<SeatRow>, sqlx::Error> {
    sqlx::query_as::<_, SeatRow>(
        "SELECT id, seat_row, seat_number FROM reservation_seats
         WHERE reservation_id = $1 ORDER BY seat_row, seat_number",
    )
    .bind(reservation_id)
    .fetch_all(pool)
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

// The reservation and its seats are saved together, if one seat is taken nothing is saved
pub async fn create_reservation(
    pool: &PgPool,
    user_id: i32,
    screening_id: i32,
    seats: &[SeatRequest],
) -> Result<ReservationRow, sqlx::Error> {
    let mut transaction = pool.begin().await?;

    let reservation = sqlx::query_as::<_, ReservationRow>(
        "INSERT INTO reservations (user_id, screening_id) VALUES ($1, $2) RETURNING *",
    )
    .bind(user_id)
    .bind(screening_id)
    .fetch_one(&mut *transaction)
    .await?;

    for seat in seats {
        sqlx::query(
            "INSERT INTO reservation_seats (reservation_id, screening_id, seat_row, seat_number)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(reservation.id)
        .bind(screening_id)
        .bind(seat.row)
        .bind(seat.number)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;
    Ok(reservation)
}

// Swaps all seats of a pending reservation for new ones
pub async fn replace_seats(
    pool: &PgPool,
    reservation: &ReservationRow,
    seats: &[SeatRequest],
) -> Result<ReservationRow, sqlx::Error> {
    let mut transaction = pool.begin().await?;

    sqlx::query("DELETE FROM reservation_seats WHERE reservation_id = $1")
        .bind(reservation.id)
        .execute(&mut *transaction)
        .await?;

    for seat in seats {
        sqlx::query(
            "INSERT INTO reservation_seats (reservation_id, screening_id, seat_row, seat_number)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(reservation.id)
        .bind(reservation.screening_id)
        .bind(seat.row)
        .bind(seat.number)
        .execute(&mut *transaction)
        .await?;
    }

    let updated_reservation = sqlx::query_as::<_, ReservationRow>(
        "UPDATE reservations SET updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(reservation.id)
    .fetch_one(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(updated_reservation)
}

// The reservation stays saved, its seats become free again
pub async fn cancel_reservation(pool: &PgPool, reservation_id: i32) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;

    sqlx::query("UPDATE reservations SET status = 'cancelled', updated_at = NOW() WHERE id = $1")
        .bind(reservation_id)
        .execute(&mut *transaction)
        .await?;

    sqlx::query(
        "UPDATE reservation_seats SET cancelled_at = NOW()
         WHERE reservation_id = $1 AND cancelled_at IS NULL",
    )
    .bind(reservation_id)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await
}

// Returns true if the seat was part of this reservation and got deleted
pub async fn delete_seat(
    pool: &PgPool,
    reservation_id: i32,
    seat_id: i32,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM reservation_seats WHERE id = $1 AND reservation_id = $2")
        .bind(seat_id)
        .bind(reservation_id)
        .execute(pool)
        .await?;
    if result.rows_affected() > 0 {
        sqlx::query("UPDATE reservations SET updated_at = NOW() WHERE id = $1")
            .bind(reservation_id)
            .execute(pool)
            .await?;
    }
    Ok(result.rows_affected() > 0)
}
