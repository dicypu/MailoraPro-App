use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// In-memory account credential store (very temporary, not secure)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct AccountCreds {
    pub email: String,
    pub password: String,
    pub host: String,
    pub port: u16,
}

pub static ACCOUNTS: Lazy<Arc<RwLock<std::collections::HashMap<String, AccountCreds>>>> =
    Lazy::new(|| Arc::new(RwLock::new(std::collections::HashMap::new())));

#[derive(Serialize, Deserialize)]
pub struct DiffRequest {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct DiffResponse {
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderCursor {
    pub name: String,
    pub uidvalidity: u32,
    pub last_uid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorToken {
    // legacy single-folder fields (kept for backward compatibility)
    pub folder: String, // INBOX or "*" when multi-folder
    pub uidvalidity: u32,
    pub last_uid: u32,
    pub modseq: Option<u64>,
    // new multi-folder support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folders: Option<Vec<FolderCursor>>, // when present, aggregate across these folders
}

pub fn encode_cursor(c: &CursorToken) -> String {
    let bytes = serde_json::to_vec(c).unwrap();
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn decode_cursor(s: &str) -> Result<CursorToken> {
    let bytes = URL_SAFE_NO_PAD.decode(s)?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(dead_code)]
pub enum ChangeItem {
    // include folder so client can fetch body from correct mailbox
    MessageAdded {
        folder: String,
        uid: u32,
        subject: String,
        from: String,
        date: Option<String>,
        size: Option<u32>,
    },
    MessageRemoved {
        uid: u32,
    },
    MessageFlagsUpdated {
        uid: u32,
        flags: Vec<String>,
    },
}

#[derive(Serialize)]
pub struct DiffResponseWire {
    pub accountId: String,
    pub since: String,
    pub next: String,
    pub changes: Vec<ChangeItem>,
}

#[allow(dead_code)]
pub async fn initial_diff_with_folder(
    account_id: &str,
    folder: &str,
    uidvalidity: u32,
    last_uid: u32,
) -> DiffResponseWire {
    let cur = CursorToken {
        folder: folder.into(),
        uidvalidity,
        last_uid,
        modseq: None,
        folders: None,
    };
    let token = encode_cursor(&cur);
    DiffResponseWire {
        accountId: account_id.into(),
        since: token.clone(),
        next: token,
        changes: vec![],
    }
}

#[allow(dead_code)]
pub async fn initial_diff(account_id: &str, uidvalidity: u32, last_uid: u32) -> DiffResponseWire {
    initial_diff_with_folder(account_id, "INBOX", uidvalidity, last_uid).await
}

#[allow(dead_code)]
pub async fn incremental_diff(
    account_id: &str,
    cursor: &CursorToken,
    new_last_uid: u32,
    new_msgs: Vec<crate::imap::sync::NewMessageMeta>,
) -> DiffResponseWire {
    let mut next_cur = cursor.clone();
    next_cur.last_uid = new_last_uid;
    let next_tok = encode_cursor(&next_cur);
    let folder_hint = cursor.folder.clone();
    let changes = new_msgs
        .into_iter()
        .map(
            |m| crate::services::diff_service::ChangeItem::MessageAdded {
                folder: folder_hint.clone(),
                uid: m.uid,
                subject: m.subject,
                from: m.from,
                date: m.date,
                size: m.size,
            },
        )
        .collect();
    DiffResponseWire {
        accountId: account_id.into(),
        since: encode_cursor(cursor),
        next: next_tok,
        changes,
    }
}

#[allow(dead_code)]
// Placeholder diff compute (not used by routes::diff which has its own handler for now)
pub async fn compute_diff(_req: DiffRequest) -> DiffResponse {
    DiffResponse { changes: vec![] }
}
