/// Unified Inbox - aggregates messages from all accounts
use axum::{extract::{State, Query}, http::StatusCode, Json};
use crate::rbac::AuthUser;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use sqlx::Row; // for try_get in dynamic row access

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UnifiedMessage {
    pub account_id: String,
    pub folder: String,
    pub uid: i64,
    pub message_id: Option<String>,
    pub subject: Option<String>,
    pub from_addr: Option<String>,
    pub to_addr: Option<String>,
    pub date: Option<String>,
    pub flags: Option<String>,
    pub size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct UnifiedInboxResponse {
    pub messages: Vec<UnifiedMessage>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct UnifiedQuery { pub limit: Option<u32>, pub offset: Option<u32>, pub unread_only: Option<bool>, pub folder: Option<String> }

/// GET /unified/inbox - Returns all INBOX messages from all accounts, sorted by date
pub async fn unified_inbox(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Query(q): Query<UnifiedQuery>
) -> Result<Json<UnifiedInboxResponse>, StatusCode> {
    let limit = q.limit.unwrap_or(100).min(500) as i64;
    let offset = q.offset.unwrap_or(0) as i64;
    let folder_orig = q.folder.unwrap_or_else(|| "INBOX".to_string());
    let folder_str = folder_orig.to_lowercase();
    let folder_cond = match folder_str.as_str() {
        "inbox" => "LOWER(m.folder) = 'inbox'".to_string(),
        "sent" => "(LOWER(m.folder) LIKE '%sent%' OR LOWER(m.folder) LIKE '%gönderilmiş%' OR LOWER(m.folder) LIKE '%g&apy%')".to_string(),
        "drafts" => "(LOWER(m.folder) LIKE '%draft%' OR LOWER(m.folder) LIKE '%taslak%')".to_string(),
        "spam" => "(LOWER(m.folder) LIKE '%spam%' OR LOWER(m.folder) LIKE '%junk%')".to_string(),
        "trash" => "(LOWER(m.folder) LIKE '%trash%' OR LOWER(m.folder) LIKE '%bin%' OR LOWER(m.folder) LIKE '%çöp%' OR LOWER(m.folder) LIKE '%&amc%')".to_string(),
        _ => format!("m.folder = '{}'", folder_orig.replace("'", "''")),
    };

    let unread_filter = if q.unread_only.unwrap_or(false) { "AND (m.flags NOT LIKE '%\\Seen%')" } else { "" };
    
    // Admin sees all, Member sees only assigned
    let snooze_filter = "AND (m.snoozed_until IS NULL OR m.snoozed_until <= datetime('now'))";

    let messages = if auth_user.role == "Admin" {
        let unread_filter_adm = if q.unread_only.unwrap_or(false) { "AND (m.flags NOT LIKE '%\\Seen%')" } else { "" };
        let sql = format!(
            "SELECT m.account_id, m.folder, m.uid, m.message_id, m.subject, m.from_addr, m.to_addr, m.date, m.flags, m.size \
             FROM messages m WHERE {} {} {} ORDER BY m.date DESC LIMIT ? OFFSET ?",
            folder_cond, unread_filter_adm, snooze_filter
        );
        sqlx::query_as::<_, UnifiedMessage>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
    } else {
        let sql = format!(
            "SELECT m.account_id, m.folder, m.uid, m.message_id, m.subject, m.from_addr, m.to_addr, m.date, m.flags, m.size \
             FROM messages m \
             JOIN user_accounts ua ON m.account_id = ua.account_id \
             WHERE {} AND ua.user_id = ? {} {} ORDER BY m.date DESC LIMIT ? OFFSET ?",
            folder_cond, unread_filter, snooze_filter
        );
        sqlx::query_as::<_, UnifiedMessage>(&sql)
            .bind(auth_user.id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&pool)
            .await
    };

    let messages = messages
        .map_err(|e| {
            tracing::error!("Failed to fetch unified inbox: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Decode encoded-word subjects if necessary
    let messages: Vec<UnifiedMessage> = messages
        .into_iter()
        .map(|mut m| {
            if let Some(ref s) = m.subject {
                if s.contains("=?") { m.subject = Some(crate::imap::sync::decode_subject(s.as_bytes())); }
            }
            m
        })
        .collect();

    let total = messages.len();
    Ok(Json(UnifiedInboxResponse { messages, total }))
}

/// GET /unified/events - Returns recent events (IN/OUT)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventRecord {
    pub id: i64,
    pub direction: String,
    pub mailbox: String,
    pub actor: Option<String>,
    pub peer: Option<String>,
    pub subject: Option<String>,
    pub ts: i64,
}

#[derive(Debug, Serialize)]
pub struct EventsResponse {
    pub events: Vec<EventRecord>,
    pub total: usize,
}

pub async fn unified_events(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
) -> Result<Json<EventsResponse>, StatusCode> {
    let events = if auth_user.role == "Admin" {
        sqlx::query_as!(
            EventRecord,
            r#"
            SELECT id as "id!", direction as "direction!", mailbox as "mailbox!", 
                   actor, peer, subject, ts as "ts!"
            FROM events
            ORDER BY ts DESC
            LIMIT 100
            "#
        )
        .fetch_all(&pool)
        .await
    } else {
         sqlx::query_as!(
            EventRecord,
            r#"
            SELECT e.id as "id!", e.direction as "direction!", e.mailbox as "mailbox!", 
                   e.actor, e.peer, e.subject, e.ts as "ts!"
            FROM events e
            JOIN user_accounts ua ON e.mailbox = ua.account_id
            WHERE ua.user_id = ?
            ORDER BY e.ts DESC
            LIMIT 100
            "#,
            auth_user.id
        )
        .fetch_all(&pool)
        .await
    };

    let events = events.map_err(|e| {
        tracing::error!("Failed to fetch events: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total = events.len();
    Ok(Json(EventsResponse { events, total }))
}

/// GET /unified/unread - Returns unread message counters
#[derive(Debug, Serialize)]
pub struct UnreadCounters { pub total_unread: i64, pub per_account: Vec<(String,i64)> }

pub async fn unified_unread(
    State(pool): State<SqlitePool>, 
    auth_user: AuthUser
) -> Result<Json<UnreadCounters>, StatusCode> {
    // Only count messages that are NOT snoozed
    let snooze_filter = "(snoozed_until IS NULL OR snoozed_until <= datetime('now'))";

    let rows = if auth_user.role == "Admin" {
         let sql = format!("SELECT account_id, COUNT(*) as c FROM messages WHERE folder='INBOX' AND (flags NOT LIKE '%\\Seen%') AND {} GROUP BY account_id", snooze_filter);
         sqlx::query(&sql)
            .fetch_all(&pool).await
    } else {
         let sql = format!("SELECT m.account_id, COUNT(*) as c FROM messages m JOIN user_accounts ua ON m.account_id = ua.account_id WHERE m.folder='INBOX' AND (m.flags NOT LIKE '%\\Seen%') AND {} AND ua.user_id = ? GROUP BY m.account_id", snooze_filter);
         sqlx::query(&sql)
            .bind(auth_user.id)
            .fetch_all(&pool).await
    };

    let rows = rows.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut per_account = Vec::new();
    let mut total = 0i64;
    for r in rows { if let (Ok(acc), Ok(c)) = (r.try_get::<String,_>("account_id"), r.try_get::<i64,_>("c")) { per_account.push((acc.clone(), c)); total += c; } }
    Ok(Json(UnreadCounters { total_unread: total, per_account }))
}
