// filepath: /mailora-hub-imap/mailora-hub-imap/src/models/message.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Message {
    pub id: i64,
    pub thread_id: i64,
    pub subject: String,
    pub body: String,
    pub sender: String,
    pub recipient: String,
    pub timestamp: String,
    pub is_read: bool,
}
