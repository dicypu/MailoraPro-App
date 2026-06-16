/// POST /test/smtp/:account_id - SMTP ile test mail gönder
use crate::smtp;
use axum::extract::Json as AxumJson;

#[derive(Debug, Deserialize)]
pub struct SmtpTestRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
}

async fn bump_metrics(pool: &SqlitePool, emails: i64, success: i64, pending: i64) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let _ = sqlx::query(
        r#"INSERT INTO metrics_snapshots (ts, emails_sent, finalize_success, finalize_pending)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(ts) DO UPDATE SET
              emails_sent = emails_sent + excluded.emails_sent,
              finalize_success = finalize_success + excluded.finalize_success,
              finalize_pending = finalize_pending + excluded.finalize_pending"#,
    )
    .bind(ts)
    .bind(emails)
    .bind(success)
    .bind(pending)
    .execute(pool)
    .await;
}

pub async fn smtp_test(
    State(pool): State<SqlitePool>,
    Path(account_id): Path<String>,
    AxumJson(req): AxumJson<SmtpTestRequest>,
) -> Json<serde_json::Value> {
    // Always return JSON on all paths
    let account = match account_service::get_account(&pool, &account_id).await {
        Ok(Some(acc)) => acc,
        Ok(None) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Account {} not found", account_id)
            }))
        }
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Database error: {}", e)
            }))
        }
    };

    let result = smtp::send_simple(
        &account.smtp_host,
        account.smtp_port,
        &account.email,
        &account.password,
        &req.to,
        &req.subject,
        &req.body,
    );

    match result {
        Ok(_) => {
            bump_metrics(&pool, 1, 0, 0).await;
            Json(serde_json::json!({"success": true, "message": "SMTP test mail gönderildi."}))
        }
        Err(e) => {
            tracing::error!("SMTP gönderim hatası: {:?}", e);
            Json(serde_json::json!({
                "success": false,
                "error": format!("SMTP gönderim hatası: {}", e)
            }))
        }
    }
}
/// IMAP Test Endpoints
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::services::{account_service, imap_test_service, message_body_service};

#[derive(Debug, Deserialize)]
pub struct TestQuery {
    pub limit: Option<u32>,
    pub folder: Option<String>,
    pub force_refresh: Option<bool>,
}

/// GET /test/connection/:account_id - Test IMAP connection
pub async fn test_connection(
    State(pool): State<SqlitePool>,
    Path(account_id): Path<String>,
) -> Result<Json<imap_test_service::ImapConnectionTestResult>, (StatusCode, String)> {
    let account = account_service::get_account(&pool, &account_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Account {} not found", account_id),
            )
        })?;

    tracing::info!("Testing IMAP connection for account: {}", account.email);

    let result = imap_test_service::test_imap_connection(&account)
        .await
        .map_err(|e| {
            tracing::error!("IMAP connection test failed: {}", e);

            // Provider-specific error messages
            let error_msg = e.to_string();
            let helpful_msg =
                if error_msg.contains("login failed") || error_msg.contains("LOGIN failed") {
                    match account.provider.as_str() {
                        "gmail" => {
                            "Gmail bağlantı hatası:\n\
                        • 2-Step Verification aktif olmalı\n\
                        • App Password kullanın: https://myaccount.google.com/apppasswords\n\
                        • IMAP aktif olmalı (Gmail Settings > Forwarding and POP/IMAP)"
                        }
                        "outlook" => {
                            "Outlook/Hotmail bağlantı hatası:\n\
                        • 2-Step Verification aktif olmalı\n\
                        • App Password kullanın: https://account.microsoft.com/security\n\
                        • IMAP aktif olmalı (Outlook.com > Settings > Sync email)\n\
                        • Host: outlook.office365.com, Port: 993"
                        }
                        "yahoo" => {
                            "Yahoo bağlantı hatası:\n\
                        • App Password kullanın: https://login.yahoo.com/account/security\n\
                        • IMAP aktif olmalı"
                        }
                        _ => &error_msg,
                    }
                } else {
                    &error_msg
                };

            (StatusCode::BAD_REQUEST, helpful_msg.to_string())
        })?;

    tracing::info!(
        "IMAP connection successful. Folders: {}, Messages: {}",
        result.folders.len(),
        result.inbox_stats.exists
    );

    Ok(Json(result))
}

/// GET /test/messages/:account_id - Fetch recent messages preview
pub async fn fetch_messages(
    State(pool): State<SqlitePool>,
    Path(account_id): Path<String>,
    Query(query): Query<TestQuery>,
) -> Result<Json<FetchMessagesResponse>, (StatusCode, String)> {
    let account = account_service::get_account(&pool, &account_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Account {} not found", account_id),
            )
        })?;

    let limit = query.limit.unwrap_or(10).min(50); // Max 50 messages
    let folder = query.folder.as_deref().unwrap_or("INBOX");

    tracing::info!(
        "Fetching {} recent messages for account: {} folder: {}",
        limit,
        account.email,
        folder
    );

    let messages = imap_test_service::fetch_recent_messages(&account, limit, folder)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch messages: {}", e);

            // Provider-specific error messages
            let error_msg = e.to_string();
            let helpful_msg = if error_msg.contains("LOGIN failed") {
                match account.provider.as_str() {
                    "gmail" => {
                        "LOGIN başarısız. Gmail için:\n\
                        1. 2-Step Verification aktif olmalı\n\
                        2. App Password oluşturun: https://myaccount.google.com/apppasswords\n\
                        3. Normal şifre yerine App Password kullanın"
                    }
                    "outlook" => {
                        "LOGIN başarısız. Outlook/Hotmail için:\n\
                        1. 2-Step Verification aktif olmalı\n\
                        2. App Password oluşturun: https://account.microsoft.com/security\n\
                        3. Normal şifre yerine App Password kullanın\n\
                        4. IMAP erişimi aktif olmalı (Settings > Sync email)"
                    }
                    "yahoo" => {
                        "LOGIN başarısız. Yahoo için:\n\
                        1. App Password oluşturun: https://login.yahoo.com/account/security\n\
                        2. Normal şifre yerine App Password kullanın"
                    }
                    _ => {
                        "LOGIN başarısız. Lütfen:\n\
                        1. Email ve şifrenizi kontrol edin\n\
                        2. IMAP erişimi aktif olmalı\n\
                        3. Büyük provider'lar için App Password gereklidir"
                    }
                }
            } else {
                &error_msg
            };

            (StatusCode::BAD_REQUEST, helpful_msg.to_string())
        })?;

    tracing::info!("Fetched {} messages", messages.len());

    Ok(Json(FetchMessagesResponse {
        account_id: account.id,
        email: account.email,
        message_count: messages.len(),
        messages,
    }))
}

/// GET /test/accounts - List all test accounts
pub async fn list_test_accounts(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<TestAccountInfo>>, (StatusCode, String)> {
    let accounts = account_service::list_accounts(&pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;

    let test_info: Vec<TestAccountInfo> = accounts
        .into_iter()
        .map(|acc| TestAccountInfo {
            id: acc.id,
            email: acc.email,
            provider: acc.provider.as_str().to_string(),
            enabled: acc.enabled,
            last_sync_ts: acc.last_sync_ts,
        })
        .collect();

    Ok(Json(test_info))
}

/// GET /test/body/:account_id/:uid - Fetch full message body
pub async fn fetch_message_body(
    State(pool): State<SqlitePool>,
    Path((account_id, uid)): Path<(String, u32)>,
    Query(query): Query<TestQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let account = account_service::get_account(&pool, &account_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Account {} not found", account_id),
            )
        })?;

    let folder = query.folder.as_deref();
    let body = message_body_service::fetch_message_body(&account, uid, folder, &pool, query.force_refresh.unwrap_or(false))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let resp = serde_json::json!({
        "uid": body.uid,
        "folder": body.folder,
        "subject": body.subject,
        "from": body.from,
        "date": body.date,
        "flags": body.flags,
        "plain_text": body.plain_text,
        "html_body": body.html_text,
        "raw_size": body.raw_size,
    });
    Ok(Json(resp))
}

#[derive(Debug, Serialize)]
pub struct FetchMessagesResponse {
    pub account_id: String,
    pub email: String,
    pub message_count: usize,
    pub messages: Vec<imap_test_service::MessagePreview>,
}

#[derive(Debug, Serialize)]
pub struct TestAccountInfo {
    pub id: String,
    pub email: String,
    pub provider: String,
    pub enabled: bool,
    pub last_sync_ts: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct SmtpSendAndAppendRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct SmtpSendAndAppendResponse {
    pub success: bool,
    pub folder: Option<String>,
    pub uid: Option<u32>,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

// New: finalize quick Sent scan for a Message-Id
#[derive(Debug, Deserialize)]
pub struct SmtpSentFinalizeQuery {
    pub message_id: String,
    pub max_scan: Option<usize>,
    // New: optional subject hint to improve search on providers like Gmail
    pub subject: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SmtpSentFinalizeResponse {
    pub success: bool,
    pub found: bool,
    pub folder: Option<String>,
    pub uid: Option<u32>,
    pub error: Option<String>,
}

/// POST /test/smtp-append/:account_id - Send email and append to Sent
pub async fn smtp_send_and_append(
    State(pool): State<SqlitePool>,
    Path(account_id): Path<String>,
    AxumJson(req): AxumJson<SmtpSendAndAppendRequest>,
) -> Json<SmtpSendAndAppendResponse> {
    use crate::services::account_service;

    let account = match account_service::get_account(&pool, &account_id).await {
        Ok(Some(acc)) => acc,
        Ok(None) => {
            return Json(SmtpSendAndAppendResponse { success: false, folder: None, uid: None, message_id: None, error: Some(format!("Account {} not found", account_id)) })
        }
        Err(e) => {
            return Json(SmtpSendAndAppendResponse { success: false, folder: None, uid: None, message_id: None, error: Some(format!("Database error: {}", e)) })
        }
    };

    // Build message
    let (msg, message_id) = match crate::smtp::build_email(&account.email, &req.to, &req.subject, &req.body) {
        Ok(v) => v,
        Err(e) => {
            return Json(SmtpSendAndAppendResponse { success: false, folder: None, uid: None, message_id: None, error: Some(format!("Build email failed: {}", e)) })
        }
    };

    // Render raw RFC822 before sending
    let raw = msg.formatted();

    // Send via SMTP
    if let Err(e) = crate::smtp::send_prebuilt(&account.smtp_host, account.smtp_port, &account.email, &account.password, &msg) {
        return Json(SmtpSendAndAppendResponse { success: false, folder: None, uid: None, message_id: Some(message_id), error: Some(format!("SMTP send failed: {}", e)) });
    }
    // bump emails_sent on success
    bump_metrics(&pool, 1, 0, 0).await;

    // Append/resolve with timeout (5s)
    let pool_clone = pool.clone();
    let acc_clone = account.clone();
    let subject_clone = req.subject.clone();
    let to_clone = req.to.clone();

    match tokio::time::timeout(std::time::Duration::from_secs(5), async {
        crate::smtp::append_to_sent(&acc_clone, &raw, &message_id, &acc_clone.email, &subject_clone).await
    }).await {
        Ok(Ok(append_res)) => {
            if let (Some(folder), Some(uid)) = (Some(append_res.folder.clone()), append_res.uid) {
                let _ = crate::services::message_sync_service::upsert_sent_message(&pool_clone, &acc_clone, &folder, uid, Some(&subject_clone), Some(&to_clone)).await;
                bump_metrics(&pool_clone, 0, 1, 0).await;
                return Json(SmtpSendAndAppendResponse { success: true, folder: Some(append_res.folder), uid: Some(uid), message_id: Some(message_id), error: None });
            } else {
                bump_metrics(&pool_clone, 0, 0, 1).await;
                // No UID yet (likely auto-Sent provider). Schedule retry loop: every 10s up to 60s
                let pool_bg = pool_clone.clone();
                let acc_bg = acc_clone.clone();
                let msg_id_bg = message_id.clone();
                let subject_bg = subject_clone.clone();
                let to_bg = to_clone.clone();
                let preferred_folder = append_res.folder.clone();
                tokio::spawn(async move {
                    use std::time::Duration;
                    for _attempt in 0..6 { // 6 * 10s = 60s
                        if let Ok(Some((folder, uid))) = crate::services::message_sync_service::quick_sync_sent_and_upsert(
                            &pool_bg,
                            &acc_bg,
                            &msg_id_bg,
                            Some(&subject_bg),
                            Some(&to_bg),
                            300,
                        ).await {
                            let _ = crate::services::message_sync_service::upsert_sent_message(&pool_bg, &acc_bg, &folder, uid, Some(&subject_bg), Some(&to_bg)).await;
                            bump_metrics(&pool_bg, 0, 1, 0).await;
                            tracing::info!(email=%acc_bg.email, uid, folder=%folder, "finalize: UID resolved after retry");
                            return;
                        }
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                    bump_metrics(&pool_bg, 0, 0, 1).await;
                    tracing::warn!(email=%acc_bg.email, mid=%msg_id_bg, folder=%preferred_folder, "finalize: UID still pending after retries");
                });
                return Json(SmtpSendAndAppendResponse { success: true, folder: Some(append_res.folder), uid: None, message_id: Some(message_id), error: Some("finalize running in background".to_string()) });
            }
        }
        Ok(Err(e)) => {
            return Json(SmtpSendAndAppendResponse { success: false, folder: None, uid: None, message_id: Some(message_id), error: Some(format!("IMAP APPEND failed: {}", e)) });
        }
        Err(_) => {
            // Timeout: finalize in background with retry loop
            let message_id_bg = message_id.clone();
            let raw_bg = raw.clone();
            let pool_bg = pool_clone.clone();
            let acc_bg = acc_clone.clone();
            let subject_bg = subject_clone.clone();
            let to_bg = to_clone.clone();
            tokio::spawn(async move {
                // First try once end-to-end append/search flow
                let mut resolved: Option<(String,u32)> = None;
                if let Ok(append_res) = crate::smtp::append_to_sent(&acc_bg, &raw_bg, &message_id_bg, &acc_bg.email, &subject_bg).await {
                    if let Some(uid) = append_res.uid {
                        let folder = append_res.folder.clone();
                        let _ = crate::services::message_sync_service::upsert_sent_message(&pool_bg, &acc_bg, &folder, uid, Some(&subject_bg), Some(&to_bg)).await;
                        bump_metrics(&pool_bg, 0, 1, 0).await;
                        tracing::info!(email=%acc_bg.email, uid, folder=%folder, "finalize: UID resolved in append_to_sent");
                        return;
                    }
                }
                // Retry loop for Gmail-like auto Sent
                use std::time::Duration;
                for _attempt in 0..6 { // 6 * 10s = 60s
                    if let Ok(Some((folder, uid))) = crate::services::message_sync_service::quick_sync_sent_and_upsert(
                        &pool_bg,
                        &acc_bg,
                        &message_id_bg,
                        Some(&subject_bg),
                        Some(&to_bg),
                        300,
                    ).await {
                        resolved = Some((folder, uid));
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
                if let Some((folder, uid)) = resolved {
                    let _ = crate::services::message_sync_service::upsert_sent_message(&pool_bg, &acc_bg, &folder, uid, Some(&subject_bg), Some(&to_bg)).await;
                    bump_metrics(&pool_bg, 0, 1, 0).await;
                    tracing::info!(email=%acc_bg.email, uid, folder=%folder, "finalize: UID resolved after retry");
                } else {
                    bump_metrics(&pool_bg, 0, 0, 1).await;
                    tracing::warn!(email=%acc_bg.email, mid=%message_id_bg, "finalize: UID still pending after retries");
                }
            });
            return Json(SmtpSendAndAppendResponse { success: true, folder: None, uid: None, message_id: Some(message_id), error: Some("append/uid resolve running in background".to_string()) });
        }
    }
}

/// GET /test/sent-finalize/:account_id?message_id=...&max_scan=...
pub async fn sent_finalize(
    State(pool): State<SqlitePool>,
    Path(account_id): Path<String>,
    Query(q): Query<SmtpSentFinalizeQuery>,
) -> Json<SmtpSentFinalizeResponse> {
    use crate::services::account_service;

    let account = match account_service::get_account(&pool, &account_id).await {
        Ok(Some(acc)) => acc,
        _ => {
            return Json(SmtpSentFinalizeResponse {
                success: false,
                found: false,
                folder: None,
                uid: None,
                error: Some("Account not found".into()),
            })
        }
    };

    let max_scan = q.max_scan.unwrap_or(200);
    match crate::services::message_sync_service::quick_sync_sent_and_upsert(
        &pool,
        &account,
        &q.message_id,
        q.subject.as_deref(),
        None,
        max_scan,
    )
    .await
    {
        Ok(Some((folder, uid))) => Json(SmtpSentFinalizeResponse {
            success: true,
            found: true,
            folder: Some(folder),
            uid: Some(uid),
            error: None,
        }),
        Ok(None) => Json(SmtpSentFinalizeResponse {
            success: true,
            found: false,
            folder: None,
            uid: None,
            error: None,
        }),
        Err(e) => Json(SmtpSentFinalizeResponse {
            success: false,
            found: false,
            folder: None,
            uid: None,
            error: Some(e.to_string()),
        }),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateAppendPolicyRequest { pub append_policy: Option<String>, pub sent_folder_hint: Option<String> }
#[derive(Debug, Serialize)]
pub struct UpdateAppendPolicyResponse { pub success: bool, pub append_policy: Option<String>, pub sent_folder_hint: Option<String>, pub error: Option<String> }

/// POST /test/update-append-policy/:account_id
pub async fn update_append_policy(State(pool): State<SqlitePool>, Path(account_id): Path<String>, AxumJson(req): AxumJson<UpdateAppendPolicyRequest>) -> Json<UpdateAppendPolicyResponse> {
    // Fallback: store in-memory only or no-op when columns don't exist
    let ap = req.append_policy.as_ref().map(|s| s.to_lowercase());
    if let Some(ref v) = ap { if !["auto","never","force"].contains(&v.as_str()) { return Json(UpdateAppendPolicyResponse { success:false, append_policy: None, sent_folder_hint: None, error: Some("invalid append_policy".into()) }); } }
    // Try update; ignore error if columns missing
    let res = sqlx::query("UPDATE accounts SET append_policy = COALESCE(?, append_policy), sent_folder_hint = COALESCE(?, sent_folder_hint) WHERE id = ?")
        .bind(ap.clone())
        .bind(req.sent_folder_hint.clone())
        .bind(&account_id)
        .execute(&pool).await;
    match res {
        Ok(_) => Json(UpdateAppendPolicyResponse { success:true, append_policy: ap, sent_folder_hint: req.sent_folder_hint.clone(), error: None }),
        Err(_) => Json(UpdateAppendPolicyResponse { success:true, append_policy: ap, sent_folder_hint: req.sent_folder_hint.clone(), error: None })
    }
}

/// Metrics snapshot
#[derive(Debug, Serialize)]
pub struct MetricsSnapshot { pub ts: i64, pub emails_sent: i64, pub finalize_success: i64, pub finalize_pending: i64 }

pub async fn metrics_snapshot(State(pool): State<SqlitePool>) -> Json<serde_json::Value> {
    // Best-effort metrics; default to 0 if table/row missing
    let emails_sent: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(emails_sent),0) FROM metrics_snapshots")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    let finalize_success: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(finalize_success),0) FROM metrics_snapshots")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    let finalize_pending: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(finalize_pending),0) FROM metrics_snapshots")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    Json(serde_json::json!({
        "success": true,
        "emails_sent": emails_sent,
        "finalize_success": finalize_success,
        "finalize_pending": finalize_pending
    }))
}
use crate::imap::folders as imap_folders;
use crate::imap::folders::FolderInfo;

/// GET /test/folders/:account_id - List folders for the account
pub async fn list_folders(
    State(pool): State<SqlitePool>,
    Path(account_id): Path<String>,
) -> Result<Json<Vec<FolderInfo>>, (StatusCode, String)> {
    let account = account_service::get_account(&pool, &account_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Account not found".to_string()))?;

    // Use stored credentials directly; if encoded, models::account should provide get_credentials
    let (email, password) = match account.get_credentials() {
        Ok(v) => v,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    let folders = imap_folders::list_mailboxes(
        &account.imap_host,
        account.imap_port,
        &email,
        &password,
    )
    .await
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(folders))
}

fn json_error(code: u16, msg: &str) -> Json<serde_json::Value> { Json(serde_json::json!({"success": false, "code": code, "error": msg})) }
