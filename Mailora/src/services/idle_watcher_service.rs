/// IDLE Watcher Service - Real-time email notifications
use anyhow::{Context, Result};
use async_imap::Session;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tokio_native_tls::{TlsConnector, TlsStream};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

use crate::models::account::Account;

#[allow(dead_code)]
type ImapSession = Session<Compat<TlsStream<TcpStream>>>;

/// Global IDLE watcher manager
pub struct IdleWatcherManager {
    watchers: Arc<RwLock<HashMap<String, IdleWatcherHandle>>>,
    event_tx: broadcast::Sender<IdleEvent>,
}

#[allow(dead_code)]
pub struct IdleWatcherHandle {
    pub account_id: String,
    cancel_tx: tokio::sync::oneshot::Sender<()>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdleEvent {
    pub account_id: String,
    pub email: String,
    pub event_type: IdleEventType,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleEventType {
    NewMessage { count: u32 },
    MessageDeleted { count: u32 },
    FlagChange,
    Connected,
    Disconnected,
    Error { message: String },
}

impl IdleWatcherManager {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            watchers: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    /// Start IDLE watcher for an account
    pub async fn start_watcher(&self, account: Account) -> Result<()> {
        let account_id = account.id.clone();
        let account_id_clone = account_id.clone(); // Clone for later use

        // Check if already running
        {
            let watchers = self.watchers.read().await;
            if watchers.contains_key(&account_id) {
                return Ok(()); // Already running
            }
        }

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        let event_tx = self.event_tx.clone();
        let email = account.email.clone();

        // Spawn watcher task
        tokio::spawn(async move {
            if let Err(e) = run_idle_watcher(account, event_tx.clone(), cancel_rx).await {
                tracing::error!("IDLE watcher error for {}: {}", email, e);
                let _ = event_tx.send(IdleEvent {
                    account_id: account_id.clone(),
                    email: email.clone(),
                    event_type: IdleEventType::Error {
                        message: e.to_string(),
                    },
                    timestamp: chrono::Utc::now().timestamp(),
                });
            }
        });

        // Store handle
        let mut watchers = self.watchers.write().await;
        watchers.insert(
            account_id_clone.clone(),
            IdleWatcherHandle {
                account_id: account_id_clone,
                cancel_tx,
            },
        );

        Ok(())
    }

    /// Stop watcher for an account
    pub async fn stop_watcher(&self, account_id: &str) -> Result<()> {
        let mut watchers = self.watchers.write().await;
        if let Some(handle) = watchers.remove(account_id) {
            let _ = handle.cancel_tx.send(());
        }
        Ok(())
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<IdleEvent> {
        self.event_tx.subscribe()
    }

    /// Get active watcher count
    pub async fn active_count(&self) -> usize {
        self.watchers.read().await.len()
    }
}

async fn run_idle_watcher(
    account: Account,
    event_tx: broadcast::Sender<IdleEvent>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    let account_id = account.id.clone();
    let email = account.email.clone();

    tracing::info!("Starting IDLE watcher for {}", email);

    // Send connected event
    let _ = event_tx.send(IdleEvent {
        account_id: account_id.clone(),
        email: email.clone(),
        event_type: IdleEventType::Connected,
        timestamp: chrono::Utc::now().timestamp(),
    });

    loop {
        // Check for cancellation
        if cancel_rx.try_recv().is_ok() {
            tracing::info!("IDLE watcher cancelled for {}", email);
            break;
        }

        match idle_session(&account, &event_tx, &account_id, &email).await {
            Ok(_) => {
                tracing::info!("IDLE session ended normally for {}", email);
            }
            Err(e) => {
                tracing::error!("IDLE session error for {}: {}", email, e);

                // Send error event
                let _ = event_tx.send(IdleEvent {
                    account_id: account_id.clone(),
                    email: email.clone(),
                    event_type: IdleEventType::Error {
                        message: e.to_string(),
                    },
                    timestamp: chrono::Utc::now().timestamp(),
                });

                // Wait before retry
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
        }
    }

    // Send disconnected event
    let _ = event_tx.send(IdleEvent {
        account_id: account_id.clone(),
        email: email.clone(),
        event_type: IdleEventType::Disconnected,
        timestamp: chrono::Utc::now().timestamp(),
    });

    Ok(())
}

async fn idle_session(
    account: &Account,
    event_tx: &broadcast::Sender<IdleEvent>,
    account_id: &str,
    email: &str,
) -> Result<()> {
    let (user, pass) = account.get_credentials()?;

    // Connect
    let tcp_stream = TcpStream::connect((&account.imap_host as &str, account.imap_port))
        .await
        .context("TCP connect failed")?;

    let tls_connector = TlsConnector::from(native_tls::TlsConnector::builder().build()?);

    let tls_stream = tls_connector
        .connect(&account.imap_host, tcp_stream)
        .await
        .context("TLS handshake failed")?;

    let client = async_imap::Client::new(tls_stream.compat());

    let mut session = client
        .login(&user, &pass)
        .await
        .map_err(|e| anyhow::anyhow!("Login failed: {}", e.0))?;

    // Select INBOX
    let mailbox = session.select("INBOX").await?;
    let last_exists = mailbox.exists;

    tracing::info!("IDLE watching {} - {} messages", email, last_exists);

    // Start IDLE (this consumes session)
    let mut idle = session.idle();
    idle.init().await?;

    // Wait returns a tuple (future, interrupt_handle)
    tracing::info!("IDLE session active for {}", email);
    let (wait_future, _interrupt) = idle.wait();
    let _idle_response = wait_future.await?;

    // Idle done returns the session back
    let mut session = idle.done().await?;

    // Check for new messages
    let mailbox = session.select("INBOX").await?;
    let new_exists = mailbox.exists;

    if new_exists > last_exists {
        let new_count = new_exists - last_exists;
        tracing::info!("New messages detected for {}: +{}", email, new_count);

        let _ = event_tx.send(IdleEvent {
            account_id: account_id.to_string(),
            email: email.to_string(),
            event_type: IdleEventType::NewMessage { count: new_count },
            timestamp: chrono::Utc::now().timestamp(),
        });
    } else if new_exists < last_exists {
        let deleted_count = last_exists - new_exists;

        let _ = event_tx.send(IdleEvent {
            account_id: account_id.to_string(),
            email: email.to_string(),
            event_type: IdleEventType::MessageDeleted {
                count: deleted_count,
            },
            timestamp: chrono::Utc::now().timestamp(),
        });
    }

    session.logout().await.ok();

    Ok(())
}
