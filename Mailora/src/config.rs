use serde::Deserialize;
use std::env;

#[derive(Deserialize)]
pub struct Config {
    pub database_url: String,
    pub imap_server: String,
    pub smtp_server: String,
    pub smtp_username: String,
    pub smtp_password: String,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let imap_server = env::var("IMAP_SERVER").expect("IMAP_SERVER must be set");
        let smtp_server = env::var("SMTP_SERVER").expect("SMTP_SERVER must be set");
        let smtp_username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME must be set");
        let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD must be set");

        Config {
            database_url,
            imap_server,
            smtp_server,
            smtp_username,
            smtp_password,
        }
    }
}
