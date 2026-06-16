use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use serde::Deserialize;
use axum::extract::Query;

use crate::services::message_sync_service::{
    sync_account_messages, sync_folder_messages, SyncStats, backfill_attachments,
};
use crate::rbac::AuthUser;

/// POST /sync/:account_id - Sync all folders for an account
pub async fn sync_account(
    State(pool): State<sqlx::SqlitePool>,
    Path(account_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Get account from DB
    let account =
        sqlx::query_as::<_, crate::models::account::Account>("SELECT * FROM accounts WHERE id = ?")
            .bind(&account_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Account not found".to_string()))?
            .with_password()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Start sync
    let stats = sync_account_messages(&pool, &account)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "account_id": account_id,
        "email": account.email,
        "stats": stats,
        "total_new": stats.iter().map(|s| s.new_messages).sum::<u32>(),
        "total_updated": stats.iter().map(|s| s.updated_messages).sum::<u32>(),
        "total_deleted": stats.iter().map(|s| s.deleted_messages).sum::<u32>(),
    })))
}

/// POST /sync/:account_id/:folder - Sync a specific folder
pub async fn sync_folder(
    State(pool): State<sqlx::SqlitePool>,
    Path((account_id, folder)): Path<(String, String)>,
) -> Result<Json<SyncStats>, (StatusCode, String)> {
    // Get account from DB
    let account =
        sqlx::query_as::<_, crate::models::account::Account>("SELECT * FROM accounts WHERE id = ?")
            .bind(&account_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Account not found".to_string()))?
            .with_password()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Start sync
    let stats = sync_folder_messages(&pool, &account, &folder)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(stats))
}

/// GET /messages/:account_id - Get synced messages for an account
pub async fn get_messages(
    State(pool): State<sqlx::SqlitePool>,
    Path(account_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    #[derive(sqlx::FromRow, serde::Serialize)]
    struct MessageRow {
        id: i64,
        uid: i64,
        folder: String,
        subject: Option<String>,
        from_addr: Option<String>,
        to_addr: Option<String>,
        date: Option<String>,
        flags: Option<String>,
        size: Option<i64>,
        has_attachments: bool,
        synced_at: String,
    }

    let messages: Vec<MessageRow> = sqlx::query_as(
        r#"
        SELECT id, uid, folder, subject, from_addr, to_addr, date, flags, size, has_attachments, synced_at
        FROM messages
        WHERE account_id = ?
        ORDER BY date DESC
        LIMIT 100
        "#,
    )
    .bind(&account_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "account_id": account_id,
        "count": messages.len(),
        "messages": messages,
    })))
}

/// GET /messages/:account_id/:folder - Get messages from a specific folder
#[derive(Debug, Deserialize)]
pub struct PageQs { pub limit: Option<u32>, pub before_uid: Option<i64>, pub unread: Option<bool>, pub attachments: Option<bool> }
pub async fn get_folder_messages(
    State(pool): State<sqlx::SqlitePool>,
    Path((account_id, folder)): Path<(String, String)>,
    Query(q): Query<PageQs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    #[derive(sqlx::FromRow, serde::Serialize)]
    struct MessageRow {
        id: i64,
        uid: i64,
        subject: Option<String>,
        from_addr: Option<String>,
        date: Option<String>,
        flags: Option<String>,
        has_attachments: bool,
    }

    let limit = q.limit.unwrap_or(100).min(200) as i64;
    let unread = q.unread.unwrap_or(false);
    let want_atts = q.attachments.unwrap_or(false);

    let mut base_sql = String::from("SELECT id, uid, subject, from_addr, date, flags, has_attachments FROM messages WHERE account_id = ? AND folder = ?");
    if unread { base_sql.push_str(" AND (flags IS NULL OR flags NOT LIKE '%\\Seen%')"); }
    if want_atts { base_sql.push_str(" AND has_attachments = 1"); }

    let messages: Vec<MessageRow> = if let Some(before) = q.before_uid {
        let mut sql = base_sql.clone();
        sql.push_str(" AND uid < ? ORDER BY uid DESC LIMIT ?");
        sqlx::query_as(&sql)
            .bind(&account_id)
            .bind(&folder)
            .bind(before)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        let mut sql = base_sql.clone();
        sql.push_str(" ORDER BY uid DESC LIMIT ?");
        sqlx::query_as(&sql)
            .bind(&account_id)
            .bind(&folder)
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    Ok(Json(json!({
        "account_id": account_id,
        "folder": folder,
        "count": messages.len(),
        "messages": messages,
    })))
}

/// GET /search - search messages (subject/from, optional filters)
#[derive(Debug, Deserialize)]
pub struct SearchQs { pub q: Option<String>, pub unread: Option<bool>, pub attachments: Option<bool>, pub folder: Option<String>, pub account_id: Option<String>, pub before_uid: Option<i64>, pub limit: Option<u32>, pub start_date: Option<String>, pub end_date: Option<String> }

pub async fn search_messages(
    State(pool): State<sqlx::SqlitePool>,
    auth_user: AuthUser,
    Query(qs): Query<SearchQs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    #[derive(sqlx::FromRow, serde::Serialize)]
    struct Row { account_id: String, folder: String, uid: i64, subject: Option<String>, from_addr: Option<String>, date: Option<String>, flags: Option<String>, has_attachments: bool, size: Option<i64>, preview: Option<String> }

    let mut sql = String::new();
    let mut args: Vec<String> = Vec::new(); // using String params for everything for simplicity

    // Determine if we are doing FTS or standard scan
    let has_query = qs.q.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);

    if has_query {
        // FTS Path
        sql.push_str("SELECT m.account_id, m.folder, m.uid, m.subject, m.from_addr, m.date, m.flags, m.has_attachments, m.size, snippet(messages_fts, -1, '<b>', '</b>', '...', 64) AS preview FROM messages m JOIN messages_fts fts ON m.id = fts.rowid ");
        // Security Join if not Admin
        if auth_user.role != "Admin" {
            sql.push_str(" JOIN user_accounts ua ON m.account_id = ua.account_id ");
        }
        sql.push_str(" WHERE messages_fts MATCH ? ");
        // Sanitize/Prepare FTS query. For now, wrap in quotes to treat as primitive search or use raw.
        // FTS5 standard syntax: space is AND.
        let raw_q = qs.q.as_ref().unwrap();
        // Simple sanitization: remove " to prevent syntax breaking for now
        let safe_q = raw_q.replace("\"", "");
        // Wrap in double quotes to do phrase search and avoid FTS syntax errors
        args.push(format!("\"{}\"", safe_q));
    } else {
        // Standard Path
        sql.push_str("SELECT m.account_id, m.folder, m.uid, m.subject, m.from_addr, m.date, m.flags, m.has_attachments, m.size, NULL AS preview FROM messages m ");
        if auth_user.role != "Admin" {
            sql.push_str(" JOIN user_accounts ua ON m.account_id = ua.account_id ");
        }
        sql.push_str(" WHERE 1=1 ");
    }

    // Common Filters
    if auth_user.role != "Admin" {
        sql.push_str(" AND ua.user_id = ? ");
        args.push(auth_user.id.to_string());
    }

    if let Some(acc) = qs.account_id.as_ref() { sql.push_str(" AND m.account_id = ?"); args.push(acc.clone()); }
    if let Some(f) = qs.folder.as_ref() { sql.push_str(" AND m.folder = ?"); args.push(f.clone()); }
    if let Some(true) = qs.unread { sql.push_str(" AND (m.flags IS NULL OR m.flags NOT LIKE '%\\Seen%')"); }
    if let Some(true) = qs.attachments { sql.push_str(" AND m.has_attachments = 1"); }
    // Start/End date...
    if let Some(sd) = qs.start_date.as_ref() { sql.push_str(" AND m.date >= ?"); args.push(sd.clone()); }
    if let Some(ed) = qs.end_date.as_ref() { sql.push_str(" AND m.date <= ?"); args.push(ed.clone()); }
    if let Some(bu) = qs.before_uid { sql.push_str(" AND m.uid < ?"); args.push(bu.to_string()); }

    sql.push_str(" ORDER BY m.date DESC LIMIT ?");
    let limit = qs.limit.unwrap_or(100).min(500) as i64;

    // Execute
    let mut q = sqlx::query_as::<_, Row>(&sql);
    for v in args { q = q.bind(v); }
    q = q.bind(limit);

    let rows = q.fetch_all(&pool).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "total": rows.len(), "messages": rows })))
}

/// POST /sync/:account_id/backfill-attachments?folder=INBOX&limit=500
#[derive(Debug, Deserialize)]
pub struct BackfillQs { pub folder: Option<String>, pub limit: Option<u32> }
pub async fn backfill_attachments_endpoint(
    State(pool): State<sqlx::SqlitePool>,
    Path(account_id): Path<String>,
    Query(q): Query<BackfillQs>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let account = sqlx::query_as::<_, crate::models::account::Account>("SELECT * FROM accounts WHERE id = ?")
        .bind(&account_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Account not found".to_string()))?
        .with_password()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let limit = q.limit.unwrap_or(500) as usize;
    let stats = backfill_attachments(&pool, &account, q.folder.as_deref(), limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "account_id": account_id, "stats": stats })))
}
