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
        jwt_secret: "test-secret".to_string(),
        notifications_url: "http://localhost:1".to_string(),
        access_token_minutes: 15,
        http_client: reqwest::Client::new(),
        cache: None,
        cache_ttl_seconds: 300,
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

fn json_request(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn health_returns_ok() {
    let request = Request::get("/health").body(Body::empty()).unwrap();
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn verify_without_token_returns_401() {
    let request = Request::get("/auth/verify").body(Body::empty()).unwrap();
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "missing or invalid token");
}

#[tokio::test]
async fn register_with_short_password_returns_400() {
    let request = json_request(
        "POST",
        "/users/register",
        json!({ "name": "Test", "email": "test@example.com", "password": "1" }),
    );
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn openapi_lists_the_endpoints() {
    let request = Request::get("/openapi.json").body(Body::empty()).unwrap();
    let (status, body) = send(app_without_database(), request).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["paths"]["/users/register"].is_object());
    assert!(body["paths"]["/auth/verify"].is_object());
}

// Needs the dev database: TEST_DB_URL=postgres://m3d_users:changeme@localhost:5432/m3d_users cargo test -- --ignored
#[tokio::test]
#[ignore]
async fn register_login_and_get_user_with_database() {
    let database_url = std::env::var("TEST_DB_URL").expect("TEST_DB_URL is not set");
    let state = test_state(&database_url);
    crate::db::create_tables(&state.pool).await;
    let app = create_router(state);

    // Unique email so the test can run more than once
    let email = format!("test{}@example.com", chrono::Utc::now().timestamp_micros());
    let register = json_request(
        "POST",
        "/users/register",
        json!({ "name": "Test", "email": email, "password": "secret123" }),
    );
    let (status, user) = send(app.clone(), register).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(user.get("passwordHash").is_none());

    let login = json_request(
        "POST",
        "/users/login",
        json!({ "email": email, "password": "secret123" }),
    );
    let (status, login_body) = send(app.clone(), login).await;
    assert_eq!(status, StatusCode::OK);
    assert!(login_body["token"].is_string());
    assert!(login_body["refreshToken"].is_string());

    let get_user = Request::get(format!("/users/{}", user["id"]))
        .body(Body::empty())
        .unwrap();
    let (status, found_user) = send(app, get_user).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(found_user["email"], email);
}
