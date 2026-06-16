use anyhow::Result;
use async_imap::Session;
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_native_tls::native_tls::TlsConnector;
use tokio_util::compat::TokioAsyncReadCompatExt; // for .next()

#[derive(Debug, Clone, serde::Serialize)]
pub struct FolderInfo {
    pub name: String,
    pub flags: Vec<String>,
}

async fn connect_and_login(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
) -> Result<Session<tokio_util::compat::Compat<tokio_native_tls::TlsStream<TcpStream>>>> {
    let tcp = TcpStream::connect((host, port)).await?;
    let tls = TlsConnector::builder().build()?;
    let tls = tokio_native_tls::TlsConnector::from(tls);
    let tls_stream = tls.connect(host, tcp).await?;
    let compat = tls_stream.compat();
    let client = async_imap::Client::new(compat);
    let session = client
        .login(user, pass)
        .await
        .map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
    Ok(session)
}

pub async fn list_mailboxes(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
) -> Result<Vec<FolderInfo>> {
    let mut session = connect_and_login(host, port, user, pass).await?;
    let mut out = Vec::new();
    if let Ok(list_stream) = session.list(None, Some("*")).await {
        let mut list = list_stream;
        while let Some(item) = list.next().await {
            let mailbox = match item {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("list item error: {e}");
                    continue;
                }
            };
            let name = mailbox.name().to_string();
            let attrs: Vec<String> = mailbox
                .attributes()
                .iter()
                .map(|a| format!("{:?}", a))
                .collect();
            out.push(FolderInfo { name, flags: attrs });
        }
    }
    let _ = session.logout().await;
    Ok(out)
}

pub fn detect_sent_candidates(names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for n in names {
        let l = n.to_lowercase();
        if l.contains("[gmail]/sent mail")
            || l.ends_with("/sent")
            || l.contains("sent items")
            || l.contains("sent messages")
        {
            out.push(n.clone());
        }
    }
    if out.is_empty() {
        out.push("Sent".into());
        out.push("Sent Items".into());
        out.push("[Gmail]/Sent Mail".into());
        out.push("Sent Messages".into());
        out.push("INBOX.Sent".into());
    }
    out
}
