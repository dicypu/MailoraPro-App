/// IMAP Connection Test Service
use anyhow::{Context, Result};
use async_imap::Session;
use tokio::net::TcpStream;
use tokio_native_tls::{TlsConnector, TlsStream};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

use crate::models::account::Account;
use crate::imap::sync::decode_subject; // RFC2047 decoder

type ImapSession = Session<Compat<TlsStream<TcpStream>>>;

/// Test IMAP connection with account credentials
pub async fn test_imap_connection(account: &Account) -> Result<ImapConnectionTestResult> {
    let (email, password) = account
        .get_credentials()
        .context("Failed to decode credentials")?;

    if account.imap_port == 143 {
        // Plain connection
        let tcp_stream = TcpStream::connect((&account.imap_host as &str, account.imap_port))
            .await
            .context(format!(
                "Failed to connect to {}:{}",
                account.imap_host, account.imap_port
            ))?;
        let client = async_imap::Client::new(tcp_stream.compat());
        let mut session = client
            .login(&email, &password)
            .await
            .map_err(|e| anyhow::anyhow!("Login failed: {}", e.0))?;

        // Get capabilities
        let capabilities = session
            .capabilities()
            .await
            .context("Failed to get capabilities")?;
        let caps: Vec<String> = capabilities.iter().map(|c| format!("{:?}", c)).collect();

        // List folders
        use futures::stream::StreamExt;
        let folders_stream = session
            .list(Some(""), Some("*"))
            .await
            .context("Failed to list folders")?;
        let folders: Vec<_> = folders_stream.collect::<Vec<_>>().await;
        let folder_names: Vec<String> = folders
            .iter()
            .filter_map(
                |f: &Result<async_imap::types::Name, async_imap::error::Error>| f.as_ref().ok(),
            )
            .map(|f| f.name().to_string())
            .collect();

        // Select INBOX to get stats
        let inbox = session
            .select("INBOX")
            .await
            .context("Failed to select INBOX")?;
        let exists = inbox.exists;
        let recent = inbox.recent;
        let uidvalidity = inbox.uid_validity.unwrap_or(0);
        let uidnext = inbox.uid_next.unwrap_or(0);

        // Logout
        session.logout().await.ok();

        return Ok(ImapConnectionTestResult {
            success: true,
            capabilities: caps,
            folders: folder_names,
            inbox_stats: InboxStats {
                exists,
                recent,
                uidvalidity,
                uidnext,
            },
        });
    } else {
        // TLS connection
        let tcp_stream = TcpStream::connect((&account.imap_host as &str, account.imap_port))
            .await
            .context(format!(
                "Failed to connect to {}:{}",
                account.imap_host, account.imap_port
            ))?;
        let tls_connector = TlsConnector::from(
            native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(false)
                .build()
                .context("Failed to build TLS connector")?,
        );
        let tls_stream = tls_connector
            .connect(&account.imap_host, tcp_stream)
            .await
            .context("TLS handshake failed")?;
        let client = async_imap::Client::new(tls_stream.compat());
        let mut session = client
            .login(&email, &password)
            .await
            .map_err(|e| anyhow::anyhow!("Login failed: {}", e.0))?;

        // Get capabilities
        let capabilities = session
            .capabilities()
            .await
            .context("Failed to get capabilities")?;
        let caps: Vec<String> = capabilities.iter().map(|c| format!("{:?}", c)).collect();

        // List folders
        use futures::stream::StreamExt;
        let folders_stream = session
            .list(Some(""), Some("*"))
            .await
            .context("Failed to list folders")?;
        let folders: Vec<_> = folders_stream.collect::<Vec<_>>().await;
        let folder_names: Vec<String> = folders
            .iter()
            .filter_map(
                |f: &Result<async_imap::types::Name, async_imap::error::Error>| f.as_ref().ok(),
            )
            .map(|f| f.name().to_string())
            .collect();

        // Select INBOX to get stats
        let inbox = session
            .select("INBOX")
            .await
            .context("Failed to select INBOX")?;
        let exists = inbox.exists;
        let recent = inbox.recent;
        let uidvalidity = inbox.uid_validity.unwrap_or(0);
        let uidnext = inbox.uid_next.unwrap_or(0);

        // Logout
        session.logout().await.ok();

        return Ok(ImapConnectionTestResult {
            success: true,
            capabilities: caps,
            folders: folder_names,
            inbox_stats: InboxStats {
                exists,
                recent,
                uidvalidity,
                uidnext,
            },
        });
    }
    // Fonksiyonun sonunda unreachable kod yok. Artık sadece iki port tipi için ayrı bloklar ve return var.
}

fn candidate_names(base: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let lower = base.to_lowercase();
    match lower.as_str() {
        "sent" => {
            out.extend(["[Gmail]/Sent Mail","Sent","Sent Items","Sent Messages","INBOX.Sent"].into_iter().map(|s| s.to_string()));
        }
        "drafts" => {
            out.extend(["[Gmail]/Drafts","Drafts","Taslaklar"].into_iter().map(|s| s.to_string()));
        }
        "spam" => {
            out.extend(["[Gmail]/Spam","Spam","Junk"].into_iter().map(|s| s.to_string()));
        }
        "trash" => {
            out.extend(["[Gmail]/Trash","Trash","Deleted Items","Bin"].into_iter().map(|s| s.to_string()));
        }
        _ => {}
    }
    // Always try base name as well (first)
    out.insert(0, base.to_string());
    out
}

/// Fetch recent messages from a specific folder with fallback mapping
pub async fn fetch_recent_messages(account: &Account, limit: u32, folder: &str) -> Result<Vec<MessagePreview>> {
    let (email, password) = account.get_credentials()?;
    let candidates = candidate_names(folder);

    if account.imap_port == 143 {
        // Plain connection
        let tcp_stream = TcpStream::connect((&account.imap_host as &str, account.imap_port)).await?;
        let client = async_imap::Client::new(tcp_stream.compat());
        let mut session = client.login(&email, &password).await.map_err(|e| anyhow::anyhow!("Login failed: {}", e.0))?;
        // Try candidates
        let mut selected = None;
        for cand in &candidates {
            match session.select(cand).await {
                Ok(mailbox) => { selected = Some((cand.clone(), mailbox)); break; }
                Err(e) => {
                    let em = format!("{e:?}");
                    if !em.contains("NONEXISTENT") && !em.to_lowercase().contains("unknown mailbox") {
                        // other error: abort early
                        return Err(anyhow::anyhow!("Folder select failed: {em}"));
                    }
                }
            }
        }
        let (_used_name, inbox) = match selected { Some(v) => v, None => { session.logout().await.ok(); return Ok(vec![]); } };
        let exists = inbox.exists;
        if exists == 0 { session.logout().await.ok(); return Ok(vec![]); }
        let end = exists;
        let start = if exists > limit { exists - limit + 1 } else { 1 };
        let range = format!("{}:{}", start, end);
        use futures::stream::StreamExt;
        let messages_stream = session.fetch(&range, "(UID ENVELOPE FLAGS)").await.context("Failed to fetch messages")?;
        let messages: Vec<_> = messages_stream.collect::<Vec<_>>().await;
        let mut previews = Vec::new();
        for msg_result in messages.iter() {
            if let Ok(msg) = msg_result { if let Some(envelope) = msg.envelope() {
                let subject = envelope.subject.as_ref().map(|b| decode_subject(b)).unwrap_or("<no subject>".to_string());
                let from = envelope.from.as_ref().and_then(|addrs| addrs.first()).and_then(|addr| {
                    let name = addr.name.as_ref().map(|n| decode_subject(n));
                    let mailbox = addr.mailbox.as_ref().and_then(|m| std::str::from_utf8(m).ok());
                    let host = addr.host.as_ref().and_then(|h| std::str::from_utf8(h).ok());
                    match (name.as_deref(), mailbox, host) {
                        (Some(n), Some(m), Some(h)) if !n.is_empty() => Some(format!("{} <{}@{}>", n, m, h)),
                        (None, Some(m), Some(h)) => Some(format!("{}@{}", m, h)),
                        _ => None,
                    }
                }).unwrap_or_else(|| "<unknown>".to_string());
                let date = envelope.date.as_ref().and_then(|d| std::str::from_utf8(d).ok()).map(|s| s.to_string());
                let flags: Vec<String> = msg.flags().map(|f| format!("{:?}", f)).collect();
                previews.push(MessagePreview { uid: msg.uid.unwrap_or(0), subject, from, date, flags });
            }}
        }
        session.logout().await.ok();
        // Sort by UID descending (newest first)
        previews.sort_by(|a, b| b.uid.cmp(&a.uid));
        Ok(previews)
    } else {
        // TLS connection
        let tcp_stream = TcpStream::connect((&account.imap_host as &str, account.imap_port)).await?;
        let tls_connector = TlsConnector::from(native_tls::TlsConnector::builder().build()?);
        let tls_stream = tls_connector.connect(&account.imap_host, tcp_stream).await?;
        let client = async_imap::Client::new(tls_stream.compat());
        let mut session = client.login(&email, &password).await.map_err(|e| anyhow::anyhow!("Login failed: {}", e.0))?;
        let mut selected = None;
        for cand in &candidates {
            match session.select(cand).await {
                Ok(mailbox) => { selected = Some((cand.clone(), mailbox)); break; }
                Err(e) => {
                    let em = format!("{e:?}");
                    if !em.contains("NONEXISTENT") && !em.to_lowercase().contains("unknown mailbox") { return Err(anyhow::anyhow!("Folder select failed: {em}")); }
                }
            }
        }
        let (_used_name, inbox) = match selected { Some(v) => v, None => { session.logout().await.ok(); return Ok(vec![]); } };
        let exists = inbox.exists;
        if exists == 0 { session.logout().await.ok(); return Ok(vec![]); }
        let end = exists;
        let start = if exists > limit { exists - limit + 1 } else { 1 };
        let range = format!("{}:{}", start, end);
        use futures::stream::StreamExt;
        let messages_stream = session.fetch(&range, "(UID ENVELOPE FLAGS)").await.context("Failed to fetch messages")?;
        let messages: Vec<_> = messages_stream.collect::<Vec<_>>().await;
        let mut previews = Vec::new();
        for msg_result in messages.iter() {
            if let Ok(msg) = msg_result { if let Some(envelope) = msg.envelope() {
                let subject = envelope.subject.as_ref().map(|b| decode_subject(b)).unwrap_or("<no subject>".to_string());
                let from = envelope.from.as_ref().and_then(|addrs| addrs.first()).and_then(|addr| {
                    let name = addr.name.as_ref().map(|n| decode_subject(n));
                    let mailbox = addr.mailbox.as_ref().and_then(|m| std::str::from_utf8(m).ok());
                    let host = addr.host.as_ref().and_then(|h| std::str::from_utf8(h).ok());
                    match (name.as_deref(), mailbox, host) {
                        (Some(n), Some(m), Some(h)) if !n.is_empty() => Some(format!("{} <{}@{}>", n, m, h)),
                        (None, Some(m), Some(h)) => Some(format!("{}@{}", m, h)),
                        _ => None,
                    }
                }).unwrap_or_else(|| "<unknown>".to_string());
                let date = envelope.date.as_ref().and_then(|d| std::str::from_utf8(d).ok()).map(|s| s.to_string());
                let flags: Vec<String> = msg.flags().map(|f| format!("{:?}", f)).collect();
                previews.push(MessagePreview { uid: msg.uid.unwrap_or(0), subject, from, date, flags });
            }}
        }
        session.logout().await.ok();
        // Sort by UID descending (newest first)
        previews.sort_by(|a, b| b.uid.cmp(&a.uid));
        Ok(previews)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ImapConnectionTestResult {
    pub success: bool,
    pub capabilities: Vec<String>,
    pub folders: Vec<String>,
    pub inbox_stats: InboxStats,
}

#[derive(Debug, serde::Serialize)]
pub struct InboxStats {
    pub exists: u32,
    pub recent: u32,
    pub uidvalidity: u32,
    pub uidnext: u32,
}

#[derive(Debug, serde::Serialize)]
pub struct MessagePreview {
    pub uid: u32,
    pub subject: String,
    pub from: String,
    pub date: Option<String>,
    pub flags: Vec<String>,
}
