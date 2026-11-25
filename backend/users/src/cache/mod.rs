pub mod user_cache;

use std::time::Duration;

use redis::aio::{ConnectionManager, ConnectionManagerConfig};

// Connects to Valkey (it uses the same protocol as Redis).
// Returns None if Valkey is not running, then the service only uses the database.
pub async fn connect(cache_url: &str) -> Option<ConnectionManager> {
    let client = match redis::Client::open(cache_url) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("warning: invalid CACHE_URL, cache is off: {error}");
            return None;
        }
    };

    // Short timeouts and no retry wait, so a broken cache never makes requests slow.
    // It still reconnects on the next request when Valkey is back.
    let config = ConnectionManagerConfig::new()
        .set_connection_timeout(Duration::from_millis(500))
        .set_response_timeout(Duration::from_millis(500))
        .set_number_of_retries(0)
        .set_max_delay(100);

    match ConnectionManager::new_with_config(client, config).await {
        Ok(connection) => {
            println!("connected to cache at {cache_url}");
            Some(connection)
        }
        Err(error) => {
            eprintln!("warning: could not connect to cache, cache is off: {error}");
            None
        }
    }
}
