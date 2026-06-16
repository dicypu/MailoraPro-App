// filepath: /mailora-hub-imap/mailora-hub-imap/src/models/attachment.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Attachment {
    pub id: i64,
    pub message_id: i64,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub data: Vec<u8>,
}
