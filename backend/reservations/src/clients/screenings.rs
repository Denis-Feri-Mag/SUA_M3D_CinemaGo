use axum::http::StatusCode;

use crate::errors::ApiError;
use crate::state::AppState;

// Asks the screenings service if the screening exists.
// A 404 means it does not exist. If the service is not running yet we only log
// a warning and allow the booking, the seat and price checks come later.
pub async fn check_screening_exists(state: &AppState, screening_id: i32) -> Result<(), ApiError> {
    let url = format!("{}/screenings/{screening_id}", state.screenings_url);

    match state.http_client.get(&url).send().await {
        Ok(response) if response.status() == StatusCode::NOT_FOUND => {
            Err(ApiError::new(StatusCode::NOT_FOUND, "screening not found"))
        }
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) => {
            eprintln!(
                "warning: screenings service answered {}, screening not checked",
                response.status()
            );
            Ok(())
        }
        Err(request_error) => {
            eprintln!(
                "warning: could not reach screenings service, screening not checked: {request_error}"
            );
            Ok(())
        }
    }
}
