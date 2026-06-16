/// IMAP IDLE watcher for real-time mail notifications
use anyhow::Result;
use async_imap::{Client, Session};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;
use tokio_util::compat::Compat;

#[allow(dead_code)]
type ImapSession = Session<Compat<TlsStream<TcpStream>>>;

/// Configuration for an IDLE watcher task
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct IdleConfig {
    pub account_id: String,
    pub host: String,
    pub port: u16,
    pub email: String,
    pub password: String,
    pub folder: String, // typically "INBOX"
}

/// Start an IDLE watcher task that monitors new mail and triggers callbacks
pub async fn start_idle_watcher(
    config: IdleConfig,
    on_new_mail: impl Fn(String, String) + Send + Sync + 'static,
) -> Result<tokio::task::JoinHandle<()>> {
    let handle = tokio::spawn(async move {
        if let Err(e) = idle_loop(config.clone(), on_new_mail).await {
            tracing::error!(account=%config.account_id, "IDLE loop failed: {e}");
        }
    });
    Ok(handle)
}

async fn idle_loop(
    config: IdleConfig,
    on_new_mail: impl Fn(String, String) + Send + Sync + 'static,
) -> Result<()> {
    let mut backoff_secs = 5;
    loop {
        match run_idle_session(&config, &on_new_mail).await {
            Ok(_) => {
                tracing::info!(account=%config.account_id, "IDLE session ended gracefully");
                backoff_secs = 5; // reset
            }
            Err(e) => {
                tracing::warn!(account=%config.account_id, backoff_secs, "IDLE error: {e}, retrying...");
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(300); // exponential backoff, max 5min
            }
        }
    }
}

async fn run_idle_session(
    config: &IdleConfig,
    on_new_mail: &(impl Fn(String, String) + Send + Sync),
) -> Result<()> {
    let mut session =
        connect_and_login(&config.host, config.port, &config.email, &config.password).await?;

    tracing::info!(account=%config.account_id, folder=%config.folder, "Selecting folder for IDLE");
    session.select(&config.folder).await?;

    loop {
        tracing::debug!(account=%config.account_id, folder=%config.folder, "Entering IDLE");
        let mut idle_handle = session.idle();

        // Wait for server notification (EXISTS, EXPUNGE, FETCH, etc.)
        // async-imap 0.9 API: idle_handle.wait() returns (Future, StopSource)
        let (idle_wait, _interrupt) = idle_handle.wait();

        match tokio::time::timeout(Duration::from_secs(29 * 60), idle_wait).await {
            Ok(Ok(_)) => {
                tracing::info!(account=%config.account_id, folder=%config.folder, "IDLE notification received");
                on_new_mail(config.account_id.clone(), config.folder.clone());
            }
            Ok(Err(e)) => {
                tracing::warn!(account=%config.account_id, "IDLE wait error: {e}");
                return Err(e.into());
            }
            Err(_) => {
                // Timeout - reconnect to avoid server disconnect
                tracing::debug!(account=%config.account_id, "IDLE timeout, reconnecting");
            }
        }

        // After notification, reconnect the session (IDLE consumes it)
        session =
            connect_and_login(&config.host, config.port, &config.email, &config.password).await?;
        session.select(&config.folder).await?;
    }
}

async fn connect_and_login(host: &str, port: u16, user: &str, pass: &str) -> Result<ImapSession> {
    let tls = native_tls::TlsConnector::builder().build()?;
    let tls = tokio_native_tls::TlsConnector::from(tls);
    let stream = TcpStream::connect((host, port)).await?;
    let tls_stream = tls.connect(host, stream).await?;
    let compat = tokio_util::compat::TokioAsyncReadCompatExt::compat(tls_stream);
    let client = Client::new(compat);

    let session = client
        .login(user, pass)
        .await
        .map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;

    Ok(session)
}
