use anyhow::Result;
use async_imap::Session;
use tokio::net::TcpStream;
use tokio_native_tls::native_tls::TlsConnector;
use tokio_util::compat::TokioAsyncReadCompatExt;
use tokio::time::{sleep, timeout, Duration};

pub struct ImapCapabilities {
    pub condstore: bool,
    pub qresync: bool,
}

pub struct ImapSession {
    pub session: Session<tokio_util::compat::Compat<tokio_native_tls::TlsStream<TcpStream>>>,
    pub caps: ImapCapabilities,
}

pub async fn connect(host: &str, port: u16, user: &str, pass: &str) -> Result<ImapSession> {
    let attempts: u32 = std::env::var("MAILORA_IMAP_RETRIES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    let mut last_err: Option<anyhow::Error> = None;
    for i in 0..attempts {
        let res = timeout(Duration::from_secs(10), async {
            let tcp = TcpStream::connect((host, port)).await?;
            let tls = TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()?; // TODO: remove danger in prod
            let tls = tokio_native_tls::TlsConnector::from(tls);
            let tls_stream = tls.connect(host, tcp).await?;
            let compat = tls_stream.compat();
            let client = async_imap::Client::new(compat);
            let session = client
                .login(user, pass)
                .await
                .map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
            Ok::<_, anyhow::Error>(session)
        })
        .await;
        match res {
            Ok(Ok(session)) => {
                let caps = ImapCapabilities {
                    condstore: false,
                    qresync: false,
                };
                return Ok(ImapSession { session, caps });
            }
            Ok(Err(e)) => last_err = Some(e),
            Err(_) => last_err = Some(anyhow::anyhow!("connect timeout")),
        }
        let backoff_ms = 200 * (1 << i);
        sleep(Duration::from_millis(backoff_ms as u64)).await;
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("IMAP connect failed")))
}
