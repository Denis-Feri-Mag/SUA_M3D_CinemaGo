use std::env;

// Settings read from env variables (or .env when running locally)
pub struct Config {
    pub port: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub notifications_url: String,
}

impl Config {
    pub fn from_env() -> Config {
        dotenvy::dotenv().ok();

        Config {
            port: env::var("PORT").unwrap_or("8080".to_string()),
            database_url: env::var("DB_URL").expect("DB_URL is not set"),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET is not set"),
            notifications_url: env::var("NOTIFICATIONS_URL")
                .unwrap_or("http://notifications:8080".to_string()),
        }
    }
}
