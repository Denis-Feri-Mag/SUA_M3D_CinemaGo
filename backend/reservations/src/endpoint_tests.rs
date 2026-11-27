// Endpoint tests that send requests straight to the router.
// Most of them stop before the database, so the pool is lazy and never connects.
// The test marked #[ignore] needs the dev database, see the README.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use crate::routes::create_router;
use crate::state::AppState;

fn test_state(database_url: &str) -> AppState {
    AppState {
        pool: PgPoolOptions::new().connect_lazy(database_url).unwrap(),
        // Nothing listens here, so the screening check only logs a warning
        screenings_url: "http://127.0.0.1:1".to_string(),
        http_client: reqwest::Client::new(),
    }
}

fn app_without_database() -> axum::Router {
    create_router(test_state("postgres://nobody@localhost:1/none"))
}

async fn send(app: axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&body_bytes).unwrap_or(Value::Null);
    (status, body)
}

// Request with the headers Traefik would set after the token check
fn user_request(
    method: &str,
    uri: &str,
    user_id: i32,
    role: &str,
    body: Option<Value>,
) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-User-Id", user_id.to_string())
        .header("X-User-Role", role);
    match body {
        Some(json_body) => builder
            .header("Content-Type", "application/json")
            .body(Body::from(json_body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn health_returns_ok() {
    let request = Request::get("/health").body(Body::empty()).unwrap();
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn openapi_lists_all_paths() {
    let request = Request::get("/openapi.json").body(Body::empty()).unwrap();
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::OK);
    for path in [
        "/reservations",
        "/reservations/{id}",
        "/reservations/{id}/seats/{seatId}",
        "/reservations/screenings/{screeningId}/seats",
    ] {
        assert!(body["paths"][path].is_object(), "missing {path}");
    }
}

#[tokio::test]
async fn list_without_user_headers_returns_401() {
    let request = Request::get("/reservations").body(Body::empty()).unwrap();
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "missing or invalid user");
}

#[tokio::test]
async fn create_without_user_headers_returns_401() {
    let request = Request::post("/reservations")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({ "screeningId": 1, "seats": [{ "row": 1, "number": 1 }] }).to_string(),
        ))
        .unwrap();
    let (status, _) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_without_seats_returns_400() {
    let request = user_request(
        "POST",
        "/reservations",
        5,
        "user",
        Some(json!({ "screeningId": 1, "seats": [] })),
    );
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "seats: must have 1 to 10 seats");
}

#[tokio::test]
async fn create_with_same_seat_twice_returns_400() {
    let request = user_request(
        "POST",
        "/reservations",
        5,
        "user",
        Some(
            json!({ "screeningId": 1, "seats": [{ "row": 2, "number": 3 }, { "row": 2, "number": 3 }] }),
        ),
    );
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "seats: the same seat is listed twice");
}

#[tokio::test]
async fn create_with_bad_screening_id_returns_400() {
    let request = user_request(
        "POST",
        "/reservations",
        5,
        "user",
        Some(json!({ "screeningId": 0, "seats": [{ "row": 1, "number": 1 }] })),
    );
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "screeningId: must be at least 1");
}

#[tokio::test]
async fn list_with_bad_page_size_returns_400() {
    let request = user_request("GET", "/reservations?pageSize=500", 5, "user", None);
    let (status, _) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_other_users_reservations_returns_403() {
    let request = user_request("GET", "/reservations?userId=6", 5, "user", None);
    let (status, _) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn bad_reservation_id_returns_400() {
    let request = user_request("GET", "/reservations/abc", 5, "user", None);
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn unknown_path_returns_404_json() {
    let request = Request::get("/nothing").body(Body::empty()).unwrap();
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not found");
}

// Needs the dev database: TEST_DB_URL=postgres://m3d_reservations:changeme@localhost:5435/m3d_reservations cargo test -- --ignored
#[tokio::test]
#[ignore]
async fn reservation_flow_with_database() {
    let database_url = std::env::var("TEST_DB_URL").expect("TEST_DB_URL is not set");
    let state = test_state(&database_url);
    crate::db::create_tables(&state.pool).await;
    let app = create_router(state);

    // Random screening id so the test can run more than once
    let screening_id = (chrono::Utc::now().timestamp_micros() % 1_000_000_000) as i32 + 1000;
    let owner_id = 41;
    let other_user_id = 42;

    let create = user_request(
        "POST",
        "/reservations",
        owner_id,
        "user",
        Some(
            json!({ "screeningId": screening_id, "seats": [{ "row": 1, "number": 1 }, { "row": 1, "number": 2 }] }),
        ),
    );
    let (status, reservation) = send(app.clone(), create).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(reservation["status"], "pending");
    assert_eq!(reservation["seats"].as_array().unwrap().len(), 2);
    let reservation_id = reservation["id"].as_i64().unwrap();

    // Same seat again is a conflict
    let taken = user_request(
        "POST",
        "/reservations",
        other_user_id,
        "user",
        Some(json!({ "screeningId": screening_id, "seats": [{ "row": 1, "number": 2 }] })),
    );
    let (status, body) = send(app.clone(), taken).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "seat already taken");

    // Someone else cannot see it
    let other_get = user_request(
        "GET",
        &format!("/reservations/{reservation_id}"),
        other_user_id,
        "user",
        None,
    );
    let (status, _) = send(app.clone(), other_get).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Change seats
    let update = user_request(
        "PUT",
        &format!("/reservations/{reservation_id}"),
        owner_id,
        "user",
        Some(json!({ "seats": [{ "row": 3, "number": 4 }, { "row": 3, "number": 5 }] })),
    );
    let (status, updated) = send(app.clone(), update).await;
    assert_eq!(status, StatusCode::OK);
    let first_seat_id = updated["seats"][0]["id"].as_i64().unwrap();

    // Remove one seat, the last one cannot be removed
    let remove_seat = user_request(
        "DELETE",
        &format!("/reservations/{reservation_id}/seats/{first_seat_id}"),
        owner_id,
        "user",
        None,
    );
    let (status, _) = send(app.clone(), remove_seat).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let taken_seats = Request::get(format!("/reservations/screenings/{screening_id}/seats"))
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(app.clone(), taken_seats).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["seats"], json!([{ "row": 3, "number": 5 }]));

    // Admin can cancel it, after that the seats are free and it cannot change
    let cancel = user_request(
        "DELETE",
        &format!("/reservations/{reservation_id}"),
        1,
        "admin",
        None,
    );
    let (status, _) = send(app.clone(), cancel).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let update_cancelled = user_request(
        "PUT",
        &format!("/reservations/{reservation_id}"),
        owner_id,
        "user",
        Some(json!({ "seats": [{ "row": 9, "number": 9 }] })),
    );
    let (status, _) = send(app.clone(), update_cancelled).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let taken_seats = Request::get(format!("/reservations/screenings/{screening_id}/seats"))
        .body(Body::empty())
        .unwrap();
    let (_, body) = send(app.clone(), taken_seats).await;
    assert_eq!(body["seats"], json!([]));

    let list = user_request("GET", "/reservations", owner_id, "user", None);
    let (status, page) = send(app.clone(), list).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page["total"].as_i64().unwrap() >= 1);
}
