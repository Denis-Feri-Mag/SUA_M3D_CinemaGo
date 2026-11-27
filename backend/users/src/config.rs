use std::env;

// Settings read from env variables (or .env when running locally)
pub struct Config {
    pub port: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub notifications_url: String,
    pub access_token_minutes: i64,
    pub cache_url: String,
    pub cache_ttl_seconds: u64,
    pub admin_email: Option<String>,
    pub admin_password: Option<String>,
    pub rate_limit_requests: u32,
    pub rate_limit_window_seconds: u64,
}

impl Config {
    pub fn from_env() -> Config {
        dotenvy::dotenv().ok();

        Config {
            port: env::var("PORT").unwrap_or("8080".to_string()),
            database_url: env::var("DB_URL").expect("DB_URL is not set"),
            jwt_secret: read_jwt_secret(optional_env("JWT_SECRET")),
            notifications_url: env::var("NOTIFICATIONS_URL")
                .unwrap_or("http://notifications:8080".to_string()),
            access_token_minutes: env::var("ACCESS_TOKEN_MINUTES")
                .unwrap_or("15".to_string())
                .parse()
                .expect("ACCESS_TOKEN_MINUTES must be a number"),
            cache_url: env::var("CACHE_URL").unwrap_or("redis://users-cache:6379".to_string()),
            cache_ttl_seconds: env::var("CACHE_TTL_SECONDS")
                .unwrap_or("300".to_string())
                .parse()
                .expect("CACHE_TTL_SECONDS must be a number"),
            admin_email: optional_env("ADMIN_EMAIL"),
            admin_password: optional_env("ADMIN_PASSWORD"),
            rate_limit_requests: optional_env("RATE_LIMIT_REQUESTS")
                .unwrap_or("10".to_string())
                .parse()
                .expect("RATE_LIMIT_REQUESTS must be a number"),
            rate_limit_window_seconds: optional_env("RATE_LIMIT_WINDOW_SECONDS")
                .unwrap_or("60".to_string())
                .parse()
                .expect("RATE_LIMIT_WINDOW_SECONDS must be a number"),
        }
    }
}

// None when the variable is missing or empty
fn optional_env(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

// Without a secret anyone could sign tokens, so the service does not start
fn read_jwt_secret(value: Option<String>) -> String {
    match check_jwt_secret(value) {
        Ok(secret) => secret,
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(1);
        }
    }
}

fn check_jwt_secret(value: Option<String>) -> Result<String, String> {
    match value {
        Some(secret) => Ok(secret),
        None => Err("JWT_SECRET is missing or empty, set it in .env".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::check_jwt_secret;

    #[test]
    fn jwt_secret_must_be_set() {
        assert!(check_jwt_secret(None).is_err());
        assert_eq!(
            check_jwt_secret(Some("secret".to_string())),
            Ok("secret".to_string())
        );
    }
}
