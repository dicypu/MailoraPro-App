// filepath: /mailora-hub-imap/mailora-hub-imap/src/db/queries.rs
use sqlx::sqlite::SqlitePool;
use sqlx::query;
use crate::models::{User, Mailbox, Thread, Message, Attachment};

pub async fn create_user(pool: &SqlitePool, email: &str) -> Result<User, sqlx::Error> {
    let user = query!("INSERT INTO users (email) VALUES (?);", email)
        .execute(pool)
        .await?;
    
    Ok(User { id: user.last_insert_rowid(), email: email.to_string() })
}

pub async fn create_mailbox(pool: &SqlitePool, user_id: i64, address: &str) -> Result<Mailbox, sqlx::Error> {
    let mailbox = query!("INSERT INTO mailboxes (user_id, address) VALUES (?, ?);", user_id, address)
        .execute(pool)
        .await?;
    
    Ok(Mailbox { id: mailbox.last_insert_rowid(), user_id, address: address.to_string() })
}

pub async fn create_thread(pool: &SqlitePool, title: &str) -> Result<Thread, sqlx::Error> {
    let thread = query!("INSERT INTO threads (title) VALUES (?);", title)
        .execute(pool)
        .await?;
    
    Ok(Thread { id: thread.last_insert_rowid(), title: title.to_string() })
}

pub async fn create_message(pool: &SqlitePool, thread_id: i64, content: &str) -> Result<Message, sqlx::Error> {
    let message = query!("INSERT INTO messages (thread_id, content) VALUES (?, ?);", thread_id, content)
        .execute(pool)
        .await?;
    
    Ok(Message { id: message.last_insert_rowid(), thread_id, content: content.to_string() })
}

pub async fn create_attachment(pool: &SqlitePool, message_id: i64, file_name: &str, data: Vec<u8>) -> Result<Attachment, sqlx::Error> {
    let attachment = query!("INSERT INTO attachments (message_id, file_name, data) VALUES (?, ?, ?);", message_id, file_name, data)
        .execute(pool)
        .await?;
    
    Ok(Attachment { id: attachment.last_insert_rowid(), message_id, file_name: file_name.to_string(), data })
}