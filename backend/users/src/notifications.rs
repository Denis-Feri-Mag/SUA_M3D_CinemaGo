use serde_json::json;

use crate::AppState;

// Asks the notifications service (Mila) to send a welcome message.
// If it is down we only log a warning, register still works.
pub async fn send_welcome(state: &AppState, user_id: i32, name: &str) {
    let url = format!("{}/notifications", state.notifications_url);
    let body = json!({
        "userId": user_id,
        "type": "welcome",
        "message": format!("Welcome to CinemaGo, {name}!"),
    });

    match state.http_client.post(&url).json(&body).send().await {
        Ok(response) if response.status().is_success() => {
            println!("welcome notification sent for user {user_id}");
        }
        Ok(response) => {
            eprintln!("warning: notifications service answered {}", response.status());
        }
        Err(request_error) => {
            eprintln!("warning: could not reach notifications service: {request_error}");
        }
    }
}
