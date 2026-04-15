/// Mailora v2 — Configuration
pub struct AppConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub max_payload_size: usize,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/mailora.db".to_string()),
            jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret".to_string()),
            max_payload_size: std::env::var("MAX_PAYLOAD_SIZE")
                .unwrap_or_else(|_| "10485760".to_string()).parse().unwrap_or(10485760),
        }
    }
}
