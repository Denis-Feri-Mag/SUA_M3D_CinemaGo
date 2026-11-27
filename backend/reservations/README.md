# Reservations

Lets users book seats for a screening. Written in Rust with Axum, sqlx and PostgreSQL (`m3d_reservations`).

## Run with docker

From the repo root:

```powershell
Copy-Item .env.example .env
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build reservations reservations-db
```

The service is on http://localhost:8084 and Swagger is on http://localhost:8084/docs.

## Run locally

Start only the database with compose, then:

```powershell
cd backend/reservations
Copy-Item .env.example .env
cargo run
```

The tables are created on startup.

## Code layout

- `src/main.rs` starts the service, `src/config.rs` reads env variables, `src/state.rs` shared state
- `src/routes.rs` router, `src/openapi.rs` Swagger spec
- `src/handlers/` endpoint handlers (reservations, health)
- `src/models/` database structs, requests (with validation rules) and responses
- `src/db/` connection pool, table creation and SQL queries
- `src/auth/` reads the logged in user from the headers Traefik sets
- `src/clients/` calls to other services (screenings)
- `src/errors.rs` JSON errors, `src/extractors.rs` JSON errors for bad bodies and paths
- `src/endpoint_tests.rs` endpoint tests

## Env variables

| Name           | What it is                                                 |
|----------------|------------------------------------------------------------|
| PORT           | Port the service listens on (default 8080)                 |
| DB_URL         | Postgres connection string for m3d_reservations (required) |
| SCREENINGS_URL | Base URL of the screenings service                         |
| RUST_LOG       | Log level (default `info,sqlx=warn`)                       |
| TEST_DB_URL    | Only for the ignored database test, see Tests              |

## Tables

- `reservations`: id, user_id, screening_id, status (`pending`, `paid`, `cancelled`), created_at, updated_at
- `reservation_seats`: id, reservation_id, screening_id, seat_row, seat_number, cancelled_at

A unique index on (screening_id, seat_row, seat_number) for seats that are not cancelled makes sure a seat can only be booked once.
Two people booking the same seat at the same time get 409 `{ "error": "seat already taken" }`.

## Endpoints

| Method | Path                                         | Token          | What it does                                  |
|--------|----------------------------------------------|----------------|-----------------------------------------------|
| GET    | /health                                      | no             | Health check                                  |
| GET    | /reservations                                | any user       | Your reservations, `?page=1&pageSize=20`      |
| GET    | /reservations/{id}                           | owner or admin | One reservation with its seats                |
| GET    | /reservations/screenings/{screeningId}/seats | no             | Seats that are already taken                  |
| POST   | /reservations                                | any user       | Book seats, starts as `pending` (201)         |
| PUT    | /reservations/{id}                           | owner or admin | Change the seats of a pending reservation     |
| DELETE | /reservations/{id}                           | owner or admin | Cancel, the seats are free again (204)        |
| DELETE | /reservations/{id}/seats/{seatId}            | owner or admin | Remove one seat from a pending reservation    |

Through Traefik the paths start with `/api`, for example `/api/reservations`.
Traefik checks the token with the users service and sends `X-User-Id` and `X-User-Role` to this service.
Without them the service returns 401, for a reservation of someone else 403. Admins can see and change all reservations.
When calling the service directly on port 8084 you can set the two headers yourself.

`GET /reservations` returns `{ items, page, pageSize, total }`, pageSize is 1 to 100. Admins see all reservations and can use `?userId=`.

Example body for `POST /reservations`:

```json
{ "screeningId": 3, "seats": [{ "row": 5, "number": 12 }, { "row": 5, "number": 13 }] }
```

`PUT /reservations/{id}` takes `{ "seats": [...] }` and replaces all seats.
A paid or cancelled reservation cannot be changed (409), and the last seat cannot be removed (cancel the reservation instead).
All errors are JSON like `{ "error": "reservation not found" }`.

## Validation

- screeningId: at least 1
- seats: 1 to 10 seats, row and number 1 to 50, the same seat only once

A bad value returns 400, for example `{ "error": "seats: must have 1 to 10 seats" }`.

## Calls to other services

Before booking the service calls `GET {SCREENINGS_URL}/screenings/{id}`.
If screenings answers 404 the reservation returns 404 `screening not found`.
If screenings is not running, it only logs a warning and the booking still works.

## Tests

```powershell
cargo test
```

Unit tests cover the seat checks and the user headers. Endpoint tests in `src/endpoint_tests.rs` call the router directly and do not need a database.
One test needs the dev database (start it with the dev menu first):

```powershell
$env:TEST_DB_URL = "postgres://m3d_reservations:changeme@localhost:5435/m3d_reservations"
cargo test -- --ignored
```

## OpenAPI

The spec is served on `/openapi.json`. The same spec is saved in `openapi.json` in this folder.
After changing endpoints, update it with:

```powershell
Invoke-WebRequest http://localhost:8084/openapi.json -OutFile openapi.json
```
