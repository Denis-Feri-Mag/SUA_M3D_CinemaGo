// Simple rate limit for login, register and refresh.
// Every client IP can call each path max_requests times per window, then it gets 429.
// Counters are in memory, so each users container counts on its own.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::errors::ApiError;
use crate::state::AppState;

pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    // key is "ip path", value is when the window started and how many requests came
    counters: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimiter {
    // max_requests 0 turns the limit off
    pub fn new(max_requests: u32, window_seconds: u64) -> RateLimiter {
        RateLimiter {
            max_requests,
            window: Duration::from_secs(window_seconds),
            counters: Mutex::new(HashMap::new()),
        }
    }

    // None means the request is allowed, Some(seconds) means wait that long
    pub fn check(&self, key: &str) -> Option<u64> {
        self.check_at(key, Instant::now())
    }

    fn check_at(&self, key: &str, now: Instant) -> Option<u64> {
        if self.max_requests == 0 {
            return None;
        }
        let mut counters = self.counters.lock().unwrap();

        // Remove old windows from time to time so the map does not grow forever
        if counters.len() > 10_000 {
            counters.retain(|_, (window_start, _)| now.duration_since(*window_start) < self.window);
        }

        let (window_start, request_count) = counters.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(*window_start) >= self.window {
            *window_start = now;
            *request_count = 0;
        }
        if *request_count >= self.max_requests {
            let time_left = self.window - now.duration_since(*window_start);
            let wait_seconds = (time_left.as_millis() as u64).div_ceil(1000).max(1);
            return Some(wait_seconds);
        }
        *request_count += 1;
        None
    }
}

// Middleware, used in routes.rs on the public auth routes
pub async fn limit_requests(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let connect_info = request.extensions().get::<ConnectInfo<SocketAddr>>();
    let key = format!(
        "{} {}",
        client_ip(request.headers(), connect_info),
        request.uri().path()
    );

    if let Some(wait_seconds) = state.rate_limiter.check(&key) {
        let mut response = ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "too many requests, try again later",
        )
        .into_response();
        response
            .headers_mut()
            .insert(RETRY_AFTER, HeaderValue::from(wait_seconds));
        return response;
    }
    next.run(request).await
}

// Behind Traefik every request comes from Traefik, so we use X-Forwarded-For.
// Traefik adds the real client address as the last one in the list.
fn client_ip(headers: &HeaderMap, connect_info: Option<&ConnectInfo<SocketAddr>>) -> String {
    if let Some(forwarded_for) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        if let Some(last_address) = forwarded_for
            .split(',')
            .map(|address| address.trim())
            .rfind(|address| !address.is_empty())
        {
            return last_address.to_string();
        }
    }
    match connect_info {
        Some(ConnectInfo(address)) => address.ip().to_string(),
        None => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_max_requests() {
        let limiter = RateLimiter::new(3, 60);
        let now = Instant::now();
        for _ in 0..3 {
            assert_eq!(limiter.check_at("1.2.3.4 /users/login", now), None);
        }
        assert_eq!(limiter.check_at("1.2.3.4 /users/login", now), Some(60));
    }

    #[test]
    fn other_ip_and_other_path_have_own_counter() {
        let limiter = RateLimiter::new(1, 60);
        let now = Instant::now();
        assert_eq!(limiter.check_at("1.1.1.1 /users/login", now), None);
        assert_eq!(limiter.check_at("2.2.2.2 /users/login", now), None);
        assert_eq!(limiter.check_at("1.1.1.1 /users/register", now), None);
        assert!(limiter.check_at("1.1.1.1 /users/login", now).is_some());
    }

    #[test]
    fn allows_again_after_the_window() {
        let limiter = RateLimiter::new(1, 60);
        let now = Instant::now();
        assert_eq!(limiter.check_at("ip /users/login", now), None);
        let later = now + Duration::from_secs(45);
        assert_eq!(limiter.check_at("ip /users/login", later), Some(15));
        let after_window = now + Duration::from_secs(60);
        assert_eq!(limiter.check_at("ip /users/login", after_window), None);
    }

    #[test]
    fn zero_turns_the_limit_off() {
        let limiter = RateLimiter::new(0, 60);
        for _ in 0..100 {
            assert_eq!(limiter.check("ip /users/login"), None);
        }
    }

    #[test]
    fn uses_last_forwarded_address() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.1, 203.0.113.7".parse().unwrap());
        assert_eq!(client_ip(&headers, None), "203.0.113.7");
        assert_eq!(client_ip(&HeaderMap::new(), None), "unknown");
    }
}
