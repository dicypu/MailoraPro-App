use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct OutboxEmail {
    pub id: String,
    pub account_id: String,
    pub to_addr: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub retries: i32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
