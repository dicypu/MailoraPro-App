// filepath: /mailora-hub-imap/mailora-hub-imap/src/models/thread.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Thread {
    pub id: i64,
    pub subject: String,
    pub user_id: i64,
    pub created_at: String,
    pub updated_at: String,
}
