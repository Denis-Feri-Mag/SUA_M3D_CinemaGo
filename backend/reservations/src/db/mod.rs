pub mod reservation_repository;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn connect(database_url: &str) -> PgPool {
    // One pool shared by all requests
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("could not connect to the database")
}

// Simple migration, runs on every start
pub async fn create_tables(pool: &PgPool) {
    // user_id and screening_id belong to other services, so there is no foreign key
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS reservations (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL,
            screening_id INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .expect("could not create the reservations table");

    // Seats are removed together with their reservation
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS reservation_seats (
            id SERIAL PRIMARY KEY,
            reservation_id INTEGER NOT NULL REFERENCES reservations(id) ON DELETE CASCADE,
            screening_id INTEGER NOT NULL,
            seat_row INTEGER NOT NULL,
            seat_number INTEGER NOT NULL,
            cancelled_at TIMESTAMPTZ
        )",
    )
    .execute(pool)
    .await
    .expect("could not create the reservation_seats table");

    // A seat can be booked only once per screening, cancelled seats do not count
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS reservation_seats_taken
            ON reservation_seats (screening_id, seat_row, seat_number)
            WHERE cancelled_at IS NULL",
    )
    .execute(pool)
    .await
    .expect("could not create the taken seats index");
}
