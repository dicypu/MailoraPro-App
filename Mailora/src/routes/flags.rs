use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;

#[derive(Deserialize)]
pub struct UpdateFlagsReq {
    pub seen: Option<bool>,
    pub flagged: Option<bool>,
    pub deleted: Option<bool>,
}

/// POST /messages/:account_id/:folder/:uid/flags
pub async fn update_flags(
    State(pool): State<SqlitePool>,
    Path((account_id, folder, uid)): Path<(String, String, u32)>,
    Json(req): Json<UpdateFlagsReq>,
) -> Json<serde_json::Value> {
    use crate::services::account_service;

    let account = match account_service::get_account(&pool, &account_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return Json(json!({"ok": false, "error": "account not found"})),
        Err(e) => return Json(json!({"ok": false, "error": format!("db error: {}", e)})),
    };

    // Build IMAP STORE command
    let mut flags_cmds: Vec<&str> = Vec::new();
    if let Some(seen) = req.seen { flags_cmds.push(if seen { "+FLAGS (\\Seen)" } else { "-FLAGS (\\Seen)" }); }
    if let Some(flagged) = req.flagged { flags_cmds.push(if flagged { "+FLAGS (\\Flagged)" } else { "-FLAGS (\\Flagged)" }); }
    if let Some(deleted) = req.deleted { flags_cmds.push(if deleted { "+FLAGS (\\Deleted)" } else { "-FLAGS (\\Deleted)" }); }

    if flags_cmds.is_empty() {
        return Json(json!({"ok": false, "error": "no-op"}));
    }

    // Apply to IMAP
    let res = async {
        let mut imap = crate::imap::conn::connect(&account.imap_host, account.imap_port, &account.email, &account.password).await?;
        let session = &mut imap.session;
        session.select(&folder).await?;
        for cmd in flags_cmds.iter() {
            use futures::StreamExt;
            if let Ok(mut stream) = session.uid_store(&uid.to_string(), cmd).await { while stream.next().await.is_some() {} }
        }
        session.expunge().await.ok();
        let _ = session.logout().await;
        anyhow::Ok(())
    }.await;

    if let Err(e) = res {
        return Json(json!({"ok": false, "error": e.to_string()}));
    }

    // Update DB flags snapshot
    let flags_json = serde_json::to_string(&{
        let mut v = Vec::new();
        if req.seen.unwrap_or(false) { v.push("\\Seen"); }
        if req.flagged.unwrap_or(false) { v.push("\\Flagged"); }
        if req.deleted.unwrap_or(false) { v.push("\\Deleted"); }
        v
    }).unwrap_or("[]".into());

    // Fallback: ensure message row exists
    let exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM messages WHERE account_id=? AND folder=? AND uid=?")
        .bind(&account_id).bind(&folder).bind(uid as i64)
        .fetch_one(&pool).await.unwrap_or(false);
    if !exists {
        let _ = sqlx::query("INSERT INTO messages (account_id, folder, uid, subject, from_addr, to_addr, date, flags, size, synced_at) VALUES (?,?,?,?,?,?,?,?,0, datetime('now'))")
            .bind(&account_id).bind(&folder).bind(uid as i64)
            .bind(Option::<String>::None) // subject
            .bind(Option::<String>::None) // from_addr
            .bind(Option::<String>::None) // to_addr
            .bind(Option::<String>::None) // date
            .bind(&flags_json)
            .execute(&pool).await;
    }

    let _ = sqlx::query(
        "UPDATE messages SET flags = ?, synced_at = datetime('now') WHERE account_id = ? AND folder = ? AND uid = ?",
    )
    .bind(flags_json)
    .bind(&account_id)
    .bind(&folder)
    .bind(uid as i64)
    .execute(&pool)
    .await;

    Json(json!({"ok": true}))
}
