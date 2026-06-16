use anyhow::Result;
use crate::models::account::Account;
use sqlx::{Row, SqlitePool};
use sqlx::Sqlite; // for query_scalar generic DB type

#[derive(Debug, serde::Serialize)]
pub struct MessageBody {
    pub uid: u32,
    pub folder: String,
    pub subject: String,
    pub from: String,
    pub date: Option<String>,
    pub flags: Vec<String>,
    pub plain_text: String,
    pub html_text: Option<String>,
    pub raw_size: usize,
}

/// Fetch body with simple cache layer in `message_bodies` table.
pub async fn fetch_message_body(account: &Account, uid: u32, folder: Option<&str>, pool: &SqlitePool, force_refresh: bool) -> Result<MessageBody> {
    let folder = folder.unwrap_or("INBOX");

    // Cache lookup (skip if force_refresh)
    if !force_refresh {
        match sqlx::query("SELECT body, html_body, subject, from_addr, date, flags FROM message_bodies WHERE account_id=? AND folder=? AND uid=?")
            .bind(&account.id)
            .bind(folder)
            .bind(uid as i64)
            .fetch_optional(pool)
            .await {
            Ok(Some(row)) => {
                tracing::info!("Cache HIT for UID {}", uid);
                match (row.try_get::<String,_>("body"), row.try_get::<Option<String>,_>("html_body")) {
                    (Ok(body), Ok(html_body_opt)) => {
                        let subject: String = row.try_get::<Option<String>,_>("subject").ok().flatten().unwrap_or_default();
                        let from: String = row.try_get::<Option<String>,_>("from_addr").ok().flatten().unwrap_or_default();
                        let date: Option<String> = row.try_get::<Option<String>,_>("date").ok().flatten();
                        let flags_json: String = row.try_get::<Option<String>,_>("flags").ok().flatten().unwrap_or_default();
                        let flags: Vec<String> = serde_json::from_str(&flags_json).unwrap_or_default();
                        return Ok(MessageBody { uid, folder: folder.to_string(), subject, from, date, flags, plain_text: body.clone(), html_text: html_body_opt, raw_size: body.len() });
                    }
                    (Err(e1), _) => tracing::warn!("Cache parse error body: {:?}", e1),
                    (_, Err(e2)) => tracing::warn!("Cache parse error html_body: {:?}", e2),
                }
            }
            Ok(None) => {
                tracing::info!("Cache MISS for UID {}", uid);
                tracing::info!(
                    "Fetching body for message {} from account: {}",
                    uid,
                    account.email
                );
            }
            Err(e) => tracing::error!("Cache DB error: {:?}", e),
        }
    }

    // IMAP fetch
    let fetched = crate::imap::sync::fetch_message_body_in(&account.imap_host, account.imap_port, &account.email, &account.password, uid, folder)
        .await?
        .ok_or_else(|| anyhow::anyhow!("message not found"))?;
    let body_text = fetched.body.clone();
    let mut html_opt = fetched.html_body.clone();
    let mut html_opt = fetched.html_body.clone();
    // Optimization: Do NOT sanitize here. The frontend uses a sandboxed iframe.
    // Backend sanitization was stripping essential email struct/styles causing blank screens.
    // if let Some(html) = html_opt.as_ref() { ... }
    // Best-effort cache write (ignore errors e.g., when table missing)
    match sqlx::query("INSERT OR REPLACE INTO message_bodies (account_id, folder, uid, body, html_body, subject, from_addr, date, flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&account.id)
        .bind(folder)
        .bind(uid as i64)
        .bind(&body_text)
        .bind(&html_opt)
        .bind(&fetched.subject)
        .bind(&fetched.from)
        .bind(&fetched.date)
        .bind("[]")
        .execute(pool)
        .await {
            Ok(_) => tracing::info!("Inserted body cache for UID {}", uid),
            Err(e) => tracing::error!("Failed to insert body cache for UID {}: {:?}", uid, e),
        }

    Ok(MessageBody { uid, folder: folder.to_string(), subject: fetched.subject, from: fetched.from, date: fetched.date, flags: fetched.flags, plain_text: body_text.clone(), html_text: html_opt, raw_size: body_text.len() })
}

/// Garbage collect old cache entries (TTL 48h) and cap total entries to max_rows.
pub async fn gc(pool: &SqlitePool, max_rows: i64) {
    // Delete older than 48h
    let _ = sqlx::query("DELETE FROM message_bodies WHERE created_at < strftime('%s','now') - 172800").execute(pool).await;
    // Cap size
    if let Ok(Some(cnt)) = sqlx::query_scalar::<Sqlite, i64>("SELECT COUNT(*) FROM message_bodies").fetch_optional(pool).await {
        if cnt > max_rows {
            let overflow = cnt - max_rows;
            let _ = sqlx::query("DELETE FROM message_bodies WHERE rowid IN (SELECT rowid FROM message_bodies ORDER BY created_at ASC LIMIT ?)")
                .bind(overflow)
                .execute(pool)
                .await;
        }
    }
}


/// Prefetch bodies for the most recent N messages in a folder that are not yet in cache.
pub async fn prefetch_recent_bodies(account: &Account, folder: &str, limit: u32, pool: &SqlitePool) -> Result<()> {
    // Find up to `limit` recent messages in this folder that don't exist in message_bodies
    let uids = sqlx::query(
        "SELECT uid FROM messages 
         WHERE account_id = ? AND folder = ? 
         AND uid NOT IN (SELECT uid FROM message_bodies WHERE account_id = ? AND folder = ?)
         ORDER BY uid DESC LIMIT ?"
    )
    .bind(&account.id)
    .bind(folder)
    .bind(&account.id)
    .bind(folder)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    if uids.is_empty() {
        return Ok(());
    }
    
    tracing::info!("Prefetching {} missing bodies for {}/{}", uids.len(), account.email, folder);

    // Connect to IMAP
    let mut imap_session = crate::imap::conn::connect(
        &account.imap_host,
        account.imap_port,
        &account.email,
        &account.password,
    ).await?;

    imap_session.session.select(folder).await?;

    // We can fetch them individually or in chunks.
    // Fetching individually to not blow up memory, since full bodies can be large.
    for row in uids {
        let uid: i64 = sqlx::Row::get(&row, "uid");
        // Using existing helper but passing the already open session
        // Actually, we can just use the existing `fetch_message_body` but it opens a new connection.
        // Let's implement an optimized fetch that uses our session.
        if let Ok(mut stream) = imap_session.session.uid_fetch(uid.to_string(), "(UID BODY.PEEK[])").await {
            use futures::StreamExt;
            if let Some(Ok(fetch)) = stream.next().await {
                let body_bytes = fetch.body().or_else(|| fetch.text()).unwrap_or(b"");
                if !body_bytes.is_empty() {
                    let mut subject = String::new();
                    let mut from = String::new();
                    let mut date_str = String::new();
                    
                    if let Some(parsed) = mail_parser::Message::parse(body_bytes) {
                        subject = parsed.subject().unwrap_or("").to_string();
                        match parsed.from() {
                            mail_parser::HeaderValue::Address(addr) => {
                                let name = addr.name.as_ref().map(|n| n.as_ref()).unwrap_or("");
                                let email = addr.address.as_ref().map(|a| a.as_ref()).unwrap_or("");
                                from = if !name.is_empty() { format!("{} <{}>", name, email) } else { email.to_string() };
                            }
                            mail_parser::HeaderValue::AddressList(list) => {
                                if let Some(addr) = list.first() {
                                    let name = addr.name.as_ref().map(|n| n.as_ref()).unwrap_or("");
                                    let email = addr.address.as_ref().map(|a| a.as_ref()).unwrap_or("");
                                    from = if !name.is_empty() { format!("{} <{}>", name, email) } else { email.to_string() };
                                }
                            }
                            _ => {}
                        }
                        if let Some(d) = parsed.date() {
                            date_str = d.to_rfc3339();
                        }
                        
                        let body_plain = parsed.body_text(0).unwrap_or(std::borrow::Cow::Borrowed("")).to_string();
                        let body_html = parsed.body_html(0).map(|s| s.to_string());
                        
                        let _ = sqlx::query("INSERT OR IGNORE INTO message_bodies (account_id, folder, uid, body, html_body, subject, from_addr, date, flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
                            .bind(&account.id)
                            .bind(folder)
                            .bind(uid)
                            .bind(&body_plain)
                            .bind(body_html)
                            .bind(&subject)
                            .bind(&from)
                            .bind(&date_str)
                            .bind("[]")
                            .execute(pool)
                            .await;
                    }
                }
            }
        }
    }
    
    let _ = imap_session.session.logout().await;
    Ok(())
}
