use axum::{extract::Query, Json};
use serde::Serialize;

use crate::imap::folders::list_mailboxes;
use crate::imap::sync::{fetch_new_since, initial_snapshot};
use crate::persist::ACCOUNT_STATE;
use crate::services::diff_service::ACCOUNTS;

#[derive(Serialize)]
pub struct StateSummary {
    pub accounts: Vec<String>,
    pub state_counts: Vec<(String, usize)>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(non_snake_case)]
pub struct DebugStateQuery {
    pub accountId: Option<String>,
}

#[derive(Serialize)]
pub struct FolderProbe {
    pub folder: String,
    pub last_uid: u32,
    pub new_last_uid: u32,
    pub incremental_count: usize,
}

#[derive(Serialize)]
pub struct ProbeResultMulti {
    pub account_id: String,
    pub host: String,
    pub email: String,
    pub results: Vec<FolderProbe>,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
pub struct ProbeQs {
    pub accountId: Option<String>,
    pub folder: Option<String>,
    pub limit: Option<u32>,
}

pub async fn state() -> Json<StateSummary> {
    let accs = {
        let r = ACCOUNTS.read().await;
        r.keys().cloned().collect::<Vec<_>>()
    };
    let counts = {
        let s = ACCOUNT_STATE.read().await;
        accs.iter()
            .map(|k| (k.clone(), s.get(k).map(|c| c.messages.len()).unwrap_or(0)))
            .collect::<Vec<_>>()
    };
    Json(StateSummary {
        accounts: accs,
        state_counts: counts,
    })
}

pub async fn probe_diff(
    Query(q): Query<ProbeQs>,
) -> Result<Json<ProbeResultMulti>, (axum::http::StatusCode, String)> {
    let account_id = q.accountId.clone().unwrap_or_else(|| "1".to_string());
    let creds = {
        let r = ACCOUNTS.read().await;
        r.get(&account_id).cloned()
    }
    .ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "account not logged in".into(),
    ))?;

    // Enumerate folders and filter out Spam
    let folders = list_mailboxes(&creds.host, creds.port, &creds.email, &creds.password)
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;
    let targets: Vec<String> = folders
        .into_iter()
        .map(|f| f.name)
        .filter(|n| {
            !n.to_lowercase().contains("spam")
                && !n.to_lowercase().contains("çöp")
                && !n.to_lowercase().contains("junk")
        })
        .collect();

    let mut results = Vec::new();
    for folder in targets {
        if let Ok(snap) = initial_snapshot(&creds.host, creds.port, &creds.email, &creds.password).await {
            match fetch_new_since(&creds.host, creds.port, &creds.email, &creds.password, snap.last_uid).await {
                Ok((new_last, added)) => results.push(FolderProbe { folder, last_uid: snap.last_uid, new_last_uid: new_last, incremental_count: added.len() }),
                Err(e) => { tracing::debug!(%folder, "probe error: {e}"); }
            }
        }
    }

    Ok(Json(ProbeResultMulti {
        account_id,
        host: creds.host,
        email: creds.email,
        results,
    }))
}
