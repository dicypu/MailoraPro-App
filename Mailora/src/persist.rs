use crate::services::diff_service::{AccountCreds, ACCOUNTS};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AccountStateCache {
    pub messages: std::collections::HashMap<u32, Vec<String>>,
}

#[derive(Serialize, Deserialize)]
struct PersistFile {
    version: u32,
    accounts: std::collections::HashMap<String, AccountCreds>,
    state: std::collections::HashMap<String, AccountStateCache>,
}

const FILE_PATH: &str = "accounts_state.json";

pub static ACCOUNT_STATE: Lazy<RwLock<std::collections::HashMap<String, AccountStateCache>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

pub async fn load_state() -> Result<()> {
    if !Path::new(FILE_PATH).exists() {
        return Ok(());
    }
    let data = fs::read(FILE_PATH)?;
    let pf: PersistFile = serde_json::from_slice(&data)?;
    if pf.version != 1 {
        return Ok(());
    }
    let mut w = ACCOUNTS.write().await;
    *w = pf.accounts;
    drop(w);
    let mut sw = ACCOUNT_STATE.write().await;
    *sw = pf.state;
    drop(sw);
    Ok(())
}

pub async fn save_state() -> Result<()> {
    let r = ACCOUNTS.read().await;
    let s = ACCOUNT_STATE.read().await;
    let pf = PersistFile {
        version: 1,
        accounts: r.clone(),
        state: s.clone(),
    };
    let data = serde_json::to_vec_pretty(&pf)?;
    fs::write(FILE_PATH, data)?;
    Ok(())
}
