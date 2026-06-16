use anyhow::Result;
use axum::{extract::Query, Json};
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::imap::folders::list_mailboxes;
use crate::services::diff_service::ChangeItem;
use crate::services::diff_service::ACCOUNTS;
use crate::services::diff_service::{
    decode_cursor, encode_cursor, CursorToken, DiffResponseWire, FolderCursor,
};

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct DiffQs {
    pub accountId: String,
    pub since: Option<String>,
    pub folder: Option<String>,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct BodyQs {
    pub accountId: String,
    pub uid: u32,
    pub folder: Option<String>,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct BodyResponse {
    pub accountId: String,
    pub uid: u32,
    pub subject: String,
    pub from: String,
    pub date: Option<String>,
    pub size: Option<u32>,
    pub flags: Vec<String>,
    pub body: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct FoldersQs {
    pub accountId: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct AttachQs {
    pub accountId: String,
    pub uid: u32,
    pub folder: Option<String>,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
pub struct DownloadQs {
    pub accountId: String,
    pub uid: u32,
    pub part: String,
    pub folder: Option<String>,
}

async fn list_all_folders_excluding_spam(
    creds: &crate::services::diff_service::AccountCreds,
) -> Result<Vec<String>, (axum::http::StatusCode, String)> {
    let folders = list_mailboxes(&creds.host, creds.port, &creds.email, &creds.password)
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;
    let names: Vec<String> = folders
        .into_iter()
        .map(|f| f.name)
        .filter(|n| {
            let l = n.to_lowercase();
            !(l.contains("spam") || l.contains("junk") || l.contains("çöp"))
        })
        .collect();
    Ok(names)
}

pub async fn diff_handler(
    Query(q): Query<DiffQs>,
) -> Result<Json<DiffResponseWire>, (axum::http::StatusCode, String)> {
    let account_id = q.accountId.clone();
    let creds_opt = {
        let store = ACCOUNTS.read().await;
        store.get(&account_id).cloned()
    };
    let creds = creds_opt.ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "account not logged in".into(),
    ))?;

    if let Some(tok) = q.since.as_ref() {
        let cur =
            decode_cursor(tok).map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;
        let multi = cur.folders.clone();
        let target_folders: Vec<String> = if let Some(fset) = multi {
            fset.into_iter().map(|f| f.name).collect()
        } else if cur.folder == "*" {
            list_all_folders_excluding_spam(&creds).await?
        } else {
            vec![q.folder.clone().unwrap_or(cur.folder.clone())]
        };
        let mut changes: Vec<ChangeItem> = Vec::new();
        let mut next_cur = cur.clone();
        let mut new_last_overall = cur.last_uid;
        // Track per-folder last_uid updates
        let mut per_folder_updates: Vec<(String, u32)> = Vec::new();
        for folder in target_folders.iter() {
            let last_for_folder = cur
                .folders
                .as_ref()
                .and_then(|fs| fs.iter().find(|f| &f.name == folder).map(|f| f.last_uid))
                .unwrap_or(cur.last_uid);
            // Use fetch_new_since for INBOX only; for other folders we need a folder-aware version (not yet available) so skip incremental for non-INBOX for now.
            let (mut new_last_f, new_msgs) = if folder == "INBOX" {
                crate::imap::sync::fetch_new_since(
                    &creds.host,
                    creds.port,
                    &creds.email,
                    &creds.password,
                    last_for_folder,
                )
                .await
                .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?
            } else {
                (last_for_folder, Vec::new())
            };
            if new_msgs.is_empty() && folder == "INBOX" {
                // no additional probing now
            }
            // Add change items
            for m in &new_msgs {
                changes.push(ChangeItem::MessageAdded {
                    folder: folder.clone(),
                    uid: m.uid,
                    subject: m.subject.clone(),
                    from: m.from.clone(),
                    date: m.date.clone(),
                    size: m.size,
                });
            }
            // Compute per-folder new_last using either the returned new_last_f or max uid from new_msgs
            if !new_msgs.is_empty() {
                if let Some(max_uid) = new_msgs.iter().map(|m| m.uid).max() {
                    if max_uid > new_last_f {
                        new_last_f = max_uid;
                    }
                }
            }
            if new_last_f < last_for_folder {
                new_last_f = last_for_folder;
            }
            per_folder_updates.push((folder.clone(), new_last_f));
            if new_last_f > new_last_overall {
                new_last_overall = new_last_f;
            }
        }
        next_cur.folder = if cur.folders.is_some() {
            "*".into()
        } else {
            cur.folder.clone()
        };
        next_cur.last_uid = new_last_overall;
        if let Some(fset) = cur.folders.as_ref() {
            let mut updated: Vec<FolderCursor> = Vec::new();
            for fc in fset.iter() {
                if let Some((_, nl)) = per_folder_updates.iter().find(|(name, _)| name == &fc.name)
                {
                    updated.push(FolderCursor {
                        name: fc.name.clone(),
                        uidvalidity: fc.uidvalidity,
                        last_uid: *nl,
                    });
                } else {
                    updated.push(FolderCursor {
                        name: fc.name.clone(),
                        uidvalidity: fc.uidvalidity,
                        last_uid: fc.last_uid,
                    });
                }
            }
            next_cur.folders = Some(updated);
        }
        let next = encode_cursor(&next_cur);
        let since = encode_cursor(&cur);
        return Ok(Json(DiffResponseWire {
            accountId: account_id,
            since,
            next,
            changes,
        }));
    }

    // Initial: build multi-folder cursor aggregating all (except Spam)
    let names = list_all_folders_excluding_spam(&creds).await?;
    let mut fcs: Vec<FolderCursor> = Vec::new();
    for name in names.iter() {
        if let Ok(snap) = crate::imap::sync::initial_snapshot(
            &creds.host,
            creds.port,
            &creds.email,
            &creds.password,
        )
        .await
        {
            fcs.push(FolderCursor {
                name: name.clone(),
                uidvalidity: snap.uidvalidity,
                last_uid: snap.last_uid,
            });
        }
    }
    let base_last = fcs.iter().map(|f| f.last_uid).max().unwrap_or(0);
    let cur = CursorToken {
        folder: "*".into(),
        uidvalidity: 0,
        last_uid: base_last,
        modseq: None,
        folders: Some(fcs),
    };
    let token = encode_cursor(&cur);
    Ok(Json(DiffResponseWire {
        accountId: account_id,
        since: token.clone(),
        next: token,
        changes: vec![],
    }))
}

pub async fn body_handler(
    Query(q): Query<BodyQs>,
) -> Result<Json<BodyResponse>, (axum::http::StatusCode, String)> {
    let creds_opt = {
        let store = crate::services::diff_service::ACCOUNTS.read().await;
        store.get(&q.accountId).cloned()
    };
    let creds = creds_opt.ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "account not logged in".into(),
    ))?;

    if let Some(folder) = q.folder.as_ref() {
        tracing::debug!(accountId=%q.accountId, %folder, uid=q.uid, "/body: direct folder fetch");
        match crate::imap::sync::fetch_message_body_in(
            &creds.host,
            creds.port,
            &creds.email,
            &creds.password,
            q.uid,
            folder,
        )
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?
        {
            Some(m) => {
                return Ok(Json(BodyResponse {
                    accountId: q.accountId,
                    uid: m.uid,
                    subject: m.subject,
                    from: m.from,
                    date: m.date,
                    size: m.size,
                    flags: m.flags,
                    body: m.body,
                }))
            }
            None => {
                return Err((
                    axum::http::StatusCode::NOT_FOUND,
                    "message not found".into(),
                ))
            }
        }
    }

    // Try INBOX first
    tracing::debug!(accountId=%q.accountId, uid=q.uid, "/body: try INBOX first");
    if let Ok(Some(m)) = crate::imap::sync::fetch_message_body_in(
        &creds.host,
        creds.port,
        &creds.email,
        &creds.password,
        q.uid,
        "INBOX",
    )
    .await
    {
        return Ok(Json(BodyResponse {
            accountId: q.accountId.clone(),
            uid: m.uid,
            subject: m.subject,
            from: m.from,
            date: m.date,
            size: m.size,
            flags: m.flags,
            body: m.body,
        }));
    }

    // Scan non-spam folders
    let folders = list_all_folders_excluding_spam(&creds).await?;
    for name in folders {
        if name == "INBOX" {
            continue;
        }
        tracing::debug!(accountId=%q.accountId, folder=%name, uid=q.uid, "/body: scanning folder");
        if let Ok(Some(m)) = crate::imap::sync::fetch_message_body_in(
            &creds.host,
            creds.port,
            &creds.email,
            &creds.password,
            q.uid,
            &name,
        )
        .await
        {
            return Ok(Json(BodyResponse {
                accountId: q.accountId.clone(),
                uid: m.uid,
                subject: m.subject,
                from: m.from,
                date: m.date,
                size: m.size,
                flags: m.flags,
                body: m.body,
            }));
        }
    }

    tracing::debug!(accountId=%q.accountId, uid=q.uid, "/body: not found in any folder");
    Err((
        axum::http::StatusCode::NOT_FOUND,
        "message not found".into(),
    ))
}

pub async fn folders_handler(
    Query(q): Query<FoldersQs>,
) -> Result<Json<Vec<crate::imap::folders::FolderInfo>>, (axum::http::StatusCode, String)> {
    let creds_opt = {
        let store = ACCOUNTS.read().await;
        store.get(&q.accountId).cloned()
    };
    let creds = creds_opt.ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "account not logged in".into(),
    ))?;
    let folders = crate::imap::folders::list_mailboxes(
        &creds.host,
        creds.port,
        &creds.email,
        &creds.password,
    )
    .await
    .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(Json(folders))
}

pub async fn attachments_handler(
    State(pool): State<sqlx::SqlitePool>,
    Query(q): Query<AttachQs>,
) -> Result<Json<Vec<crate::imap::sync::AttachmentMeta>>, (StatusCode, String)> {
    // First try in-memory creds (login session)
    let creds_opt = {
        let store = ACCOUNTS.read().await;
        store.get(&q.accountId).cloned()
    };
    let creds = if let Some(c) = creds_opt {
        c
    } else {
        // Fallback to DB account
        let account = crate::services::account_service::get_account(&pool, &q.accountId)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "account not found".into()))?;
        let (email, password) = account
            .get_credentials()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        crate::services::diff_service::AccountCreds {
            email,
            password,
            host: account.imap_host,
            port: account.imap_port,
        }
    };
    let req_folder = q.folder.as_deref().unwrap_or("INBOX");

    // Try requested folder first
    let mut atts = crate::imap::sync::list_attachments(
        &creds.host,
        creds.port,
        &creds.email,
        &creds.password,
        req_folder,
        q.uid,
    )
    .await
    .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let found_folder = req_folder.to_string();

    // Persist to DB: resolve message_id then replace attachments, using the found folder
    if !atts.is_empty() {
        let msg_row = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM messages WHERE account_id = ? AND folder = ? AND uid = ?",
        )
        .bind(&q.accountId)
        .bind(&found_folder)
        .bind(q.uid as i64)
        .fetch_optional(&pool)
        .await
        .map_err(int_err)?;

        if let Some(msg_id) = msg_row {
            // Replace all attachments for this message_id
            let _ = sqlx::query("DELETE FROM attachments WHERE message_id = ?")
                .bind(msg_id)
                .execute(&pool)
                .await;
            for a in &atts {
                let _ = sqlx::query(
                    r#"INSERT INTO attachments(message_id, filename, content_type, size, content_id, is_inline, data, file_path)
                        VALUES(?,?,?,?,?,0,NULL,NULL)"#,
                )
                .bind(msg_id)
                .bind(a.filename.as_deref())
                .bind(a.content_type.as_deref())
                .bind(a.size.map(|v| v as i64))
                .bind(Option::<String>::None)
                .execute(&pool)
                .await;
            }
            let has_any = !atts.is_empty();
            let _ = sqlx::query("UPDATE messages SET has_attachments = ? WHERE id = ?")
                .bind(if has_any { 1 } else { 0 })
                .bind(msg_id)
                .execute(&pool)
                .await;
        }
    }

    Ok(Json(atts))
}

pub async fn download_attachment(
    State(pool): State<sqlx::SqlitePool>,
    Query(q): Query<DownloadQs>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let creds_opt = {
        let store = ACCOUNTS.read().await;
        store.get(&q.accountId).cloned()
    };
    let creds = if let Some(c) = creds_opt {
        c
    } else {
        let account = crate::services::account_service::get_account(&pool, &q.accountId)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "account not found".into()))?;
        let (email, password) = account
            .get_credentials()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        crate::services::diff_service::AccountCreds {
            email,
            password,
            host: account.imap_host,
            port: account.imap_port,
        }
    };
    let req_folder = q.folder.as_deref().unwrap_or("INBOX");

    // Try requested folder first
    if let Ok(Some((bytes, ctype, filename))) = crate::imap::sync::fetch_attachment_part(
        &creds.host,
        creds.port,
        &creds.email,
        &creds.password,
        req_folder,
        q.uid,
        &q.part,
    )
    .await
    {
        let mut resp = axum::http::Response::builder().status(200);
        if let Some(ct) = ctype { resp = resp.header(axum::http::header::CONTENT_TYPE, ct); }
        if let Some(fname) = filename {
            resp = resp.header(
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", fname),
            );
        }
        return Ok(resp.body(axum::body::Body::from(bytes)).unwrap());
    }

    // Fallback: scan other folders
    let folders = list_all_folders_excluding_spam(&creds).await?;
    for f in folders.iter() {
        if f == req_folder { continue; }
        if let Ok(Some((bytes, ctype, filename))) = crate::imap::sync::fetch_attachment_part(
            &creds.host,
            creds.port,
            &creds.email,
            &creds.password,
            f,
            q.uid,
            &q.part,
        )
        .await
        {
            let mut resp = axum::http::Response::builder().status(200);
            if let Some(ct) = ctype { resp = resp.header(axum::http::header::CONTENT_TYPE, ct); }
            if let Some(fname) = filename {
                resp = resp.header(
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", fname),
                );
            }
            return Ok(resp.body(axum::body::Body::from(bytes)).unwrap());
        }
    }

    Err((StatusCode::NOT_FOUND, "part not found".into()))
}

fn int_err<E: std::fmt::Display>(e: E) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
