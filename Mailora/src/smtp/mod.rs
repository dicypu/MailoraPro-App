use lettre::transport::smtp::client::{TlsParameters, Tls};
use lettre::transport::smtp::authentication::Mechanism;
use tracing_subscriber::FmtSubscriber;
pub fn gmail_smtp_test() -> anyhow::Result<()> {
    // 0) Debug log aç (lettre diyaloğunu görmek için)
    let sub = FmtSubscriber::builder()
        .with_env_filter("info,lettre=trace")
        .finish();
    tracing::subscriber::set_global_default(sub).ok();

    // 1) Kimlik bilgilerini **temizle**
    let user_raw = std::env::var("GMAIL_USER")?;
    let pass_raw = std::env::var("GMAIL_APP_PASS")?;
    let user = user_raw.trim().to_string();
    let pass = pass_raw.split_whitespace().collect::<String>();
    println!(
        "user='{}' (len {})  app_pass_len={}",
        user,
        user.len(),
        pass.len()
    );

    let creds = Credentials::new(user.clone(), pass.clone());
    let tls = TlsParameters::builder("smtp.gmail.com".into()).build()?;

    // 2) STARTTLS + tek mekanizma (LOGIN)
    let mailer = SmtpTransport::starttls_relay("smtp.gmail.com")?
        .hello_name(ClientId::Domain("mailora.local".into()))
        .tls(Tls::Required(tls))
        .authentication(vec![Mechanism::Login])
        .credentials(creds)
        .build();

    // 3) From == Gmail adresinle başla
    let email = Message::builder()
        .from(user.parse()?)
        .to(user.parse()?)
        .subject("gmail starttls test")
        .body(String::from("hi"))?;

    mailer.send(&email)?;
    Ok(())
}
use lettre::transport::smtp::extension::ClientId;
/// filepath: /mailora-hub-imap/mailora-hub-imap/src/smtp/mod.rs
use anyhow::Result;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;

pub struct SmtpClient {
    smtp_transport: SmtpTransport,
}

impl SmtpClient {
    pub fn new() -> Self {
        let smtp_server = env::var("SMTP_SERVER").expect("SMTP_SERVER must be set");
        let smtp_username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME must be set");
        let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD must be set");

        let creds = Credentials::new(smtp_username, smtp_password);

        let smtp_transport = SmtpTransport::relay(&smtp_server)
            .expect("Failed to create SMTP relay")
            .credentials(creds)
            .build();

        SmtpClient { smtp_transport }
    }

    pub fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), lettre::transport::smtp::Error> {
        let email = Message::builder()
            .from("no-reply@example.com".parse().unwrap())
            .to(to.parse().unwrap())
            .subject(subject)
            .body(body.to_string())
            .unwrap();

        self.smtp_transport.send(&email)?;
        Ok(())
    }
}

pub fn send_simple(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<()> {
    use lettre::{
        transport::smtp::{
            authentication::{Credentials, Mechanism},
            client::{Tls, TlsParameters},
        },
        Message, SmtpTransport, Transport,
    };
    use std::net::IpAddr;
    use std::time::Duration;

    // Trim whitespace that may sneak in from copied app passwords
    let clean_password: String = password.chars().filter(|c| !c.is_whitespace()).collect();
    let creds = Credentials::new(username.to_string(), clean_password);

    let tls = TlsParameters::builder(host.into())
        .dangerous_accept_invalid_certs(true)
        .build()?;

    let mut builder = match SmtpTransport::relay(host) {
        Ok(b) => b,
        Err(_) => SmtpTransport::builder_dangerous(host),
    };

    let client_id = std::env::var("SMTP_HELLO_NAME")
        .ok()
        .and_then(|val| match val.parse::<IpAddr>() {
            Ok(ip) => Some(ClientId::Domain(ip.to_string())), // replaced new
            Err(_) => Some(ClientId::Domain(val)),
        })
        .unwrap_or_else(|| {
            host.parse::<IpAddr>()
                .map(|ip| ClientId::Domain(ip.to_string())) // replaced new
                .unwrap_or_else(|_| ClientId::Domain(host.to_string()))
        });

    builder = builder
        .port(port)
        .hello_name(client_id)
        .authentication(vec![Mechanism::Plain, Mechanism::Login])
        .credentials(creds)
        .timeout(Some(Duration::from_secs(20)));

    let builder = if port == 465 {
        builder.tls(Tls::Wrapper(tls))
    } else {
        builder.tls(Tls::Required(tls))
    };

    let mailer = builder.build();

    let from_addr = username.parse()?;
    let to_addr = to.parse()?;
    let email = Message::builder()
        .from(from_addr)
        .to(to_addr)
        .subject(subject)
        .body(body.to_string())?;

    match mailer.send(&email) {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!("SMTP gönderim başarısız: {:?}", e);
            Err(e.into())
        }
    }
}

/// Build a Message with explicit Message-Id. Returns (message, message_id)
pub fn build_email(from: &str, to: &str, subject: &str, body: &str) -> Result<(Message, String)> {
    use lettre::message::Mailbox;
    use lettre::message::header::MessageId;
    use uuid::Uuid;

    let from_mb: Mailbox = from.parse()?;
    let to_mb: Mailbox = to.parse()?;
    let domain = from.split('@').nth(1).unwrap_or("mailora.local");
    let msg_id_value = format!("{}@{}", Uuid::new_v4(), domain);

    let builder = Message::builder()
        .from(from_mb)
        .to(to_mb)
        .subject(subject)
        .header(MessageId::from(msg_id_value.clone()));

    let message = builder.body(body.to_string())?;
    Ok((message, msg_id_value))
}

/// Send a prebuilt Message via lettre
pub fn send_prebuilt(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    msg: &Message,
) -> Result<()> {
    use lettre::transport::smtp::{authentication::Mechanism, client::{Tls, TlsParameters}};

    let clean_password: String = password.chars().filter(|c| !c.is_whitespace()).collect();
    let creds = Credentials::new(username.to_string(), clean_password);

    let tls = TlsParameters::builder(host.into())
        .dangerous_accept_invalid_certs(true)
        .build()?;

    let mut builder = match SmtpTransport::relay(host) {
        Ok(b) => b,
        Err(_) => SmtpTransport::builder_dangerous(host),
    };

    let client_id = std::env::var("SMTP_HELLO_NAME")
        .ok()
        .map(|val| ClientId::Domain(val))
        .unwrap_or_else(|| ClientId::Domain(host.to_string()));

    builder = builder
        .port(port)
        .hello_name(client_id)
        .authentication(vec![Mechanism::Plain, Mechanism::Login])
        .credentials(creds);

    let builder = if port == 465 { builder.tls(Tls::Wrapper(tls)) } else { builder.tls(Tls::Required(tls)) };

    let mailer = builder.build();
    mailer.send(msg)?;
    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AppendResult {
    pub folder: String,
    pub uid: Option<u32>,
}

/// Append raw RFC822 to a Sent-like folder and resolve UID via Message-Id search
pub async fn append_to_sent(
    account: &crate::models::account::Account,
    raw: &[u8],
    message_id: &str,
    from_addr: &str,
    subject: &str,
) -> Result<AppendResult> {
    use crate::imap::conn;

    let mut imap = conn::connect(&account.imap_host, account.imap_port, &account.email, &account.password).await?;
    let session = &mut imap.session;

    // Collect Sent candidates
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(list_stream) = session.list(None, Some("*" )).await {
        use futures::StreamExt;
        let mut names = Vec::new();
        let mut s = list_stream;
        while let Some(item) = s.next().await { if let Ok(m) = item { names.push(m.name().to_string()); } }
        candidates = crate::imap::folders::detect_sent_candidates(&names);
    }
    if candidates.is_empty() {
        candidates = vec![
            "[Gmail]/Sent Mail".into(),
            "Sent".into(),
            "Sent Items".into(),
            "Sent Messages".into(),
            "INBOX.Sent".into(),
        ];
    }

    // Prefer Gmail special-use when provider is gmail
    if account.provider.as_str() == "gmail" {
        if let Some(pos) = candidates.iter().position(|f| f == "[Gmail]/Sent Mail") {
            if pos != 0 { let f = candidates.remove(pos); candidates.insert(0, f); }
        }
    }

    // Providers that auto-save Sent: do not APPEND unless policy overrides
    let policy = account.append_policy_enum();
    let append_allowed = match policy {
        crate::models::account::AppendPolicy::Never => false,
        crate::models::account::AppendPolicy::Force => true,
        crate::models::account::AppendPolicy::Auto => account.provider.as_str() != "gmail",
    };
    // If user provided sent_folder_hint, force it to front
    if let Some(hint) = account.sent_folder_hint.as_ref() {
        if let Some(pos) = candidates.iter().position(|f| f == hint) { let f = candidates.remove(pos); candidates.insert(0,f); }
        else { candidates.insert(0, hint.clone()); }
    }

    // Adaptive retry/backoff
    let (max_attempts, base_ms) = if account.provider.as_str() == "gmail" { (5u32, 250u64) } else { (10u32, 300u64) };

    // Helper: search for the message across candidates
    let mid_raw = message_id.trim_matches(['<','>']);
    let header_variants = vec![
        format!("HEADER Message-ID \"{}\"", mid_raw),
        format!("HEADER Message-ID \"<{}>\"", mid_raw),
        format!("HEADER Message-Id \"{}\"", mid_raw),
        format!("HEADER Message-Id \"<{}>\"", mid_raw),
    ];
    let gmail_variants = vec![
        format!("X-GM-RAW \"rfc822msgid:{}\"", mid_raw),
        format!("X-GM-RAW \"rfc822msgid:\\<{}\\>\"", mid_raw),
    ];
    let fallback_variants = vec![
        format!("SUBJECT \"{}\" FROM \"{}\"", subject, from_addr),
    ];

    let mut found_folder: Option<String> = None;
    let mut uid_opt: Option<u32> = None;

    'outer: for attempt in 0..max_attempts {
        for cand in candidates.iter() {
            let _ = session.select(cand).await;
            // 1) Standard header variants
            for q in &header_variants {
                if let Ok(uids) = session.uid_search(q).await {
                    if !uids.is_empty() { found_folder = Some(cand.clone()); uid_opt = uids.iter().copied().max(); break 'outer; }
                }
            }
            // 2) Gmail-specific search by rfc822msgid
            for q in &gmail_variants {
                if let Ok(uids) = session.uid_search(q).await {
                    if !uids.is_empty() { found_folder = Some(cand.clone()); uid_opt = uids.iter().copied().max(); break 'outer; }
                }
            }
            // 3) Fallback: subject + from (prefer non-Gmail)
            if account.provider.as_str() != "gmail" {
                for q in &fallback_variants {
                    if let Ok(uids) = session.uid_search(q).await {
                        if !uids.is_empty() { found_folder = Some(cand.clone()); uid_opt = uids.iter().copied().max(); break 'outer; }
                    }
                }
            }
        }
        // Backoff to allow server to place auto-saved Sent copy (if any)
        tokio::time::sleep(std::time::Duration::from_millis(base_ms + (attempt as u64) * base_ms)).await;
    }

    // If found without APPEND, set Seen and return
    if let (Some(folder), Some(uid)) = (found_folder.clone(), uid_opt) {
        let _ = session.uid_store(&uid.to_string(), "+FLAGS (\\Seen)").await;
        let _ = session.logout().await;
        return Ok(AppendResult { folder, uid: Some(uid) });
    }

    // Not found: perform APPEND to first accepting candidate
    if append_allowed {
        let mut appended_folder: Option<String> = None;
        for cand in candidates.iter() {
            let _ = session.select(cand).await; // best-effort
            match session.append(cand, raw).await {
                Ok(()) => { appended_folder = Some(cand.clone()); break; }
                Err(e) => { tracing::debug!(folder = %cand, error = %e, "APPEND failed, trying next candidate"); }
            }
        }
        let folder = appended_folder.ok_or_else(|| anyhow::anyhow!("No Sent-like folder accepted APPEND"))?;

        // Try to parse APPENDUID via subsequent SEARCH (async-imap does not return it today)
        // Fast path: immediately search Message-Id in the appended folder
        let _ = session.select(&folder).await;
        for q in &header_variants { if let Ok(uids) = session.uid_search(q).await { if let Some(uid) = uids.iter().copied().max() { uid_opt = Some(uid); break; } } }
        if uid_opt.is_none() { for q in &gmail_variants { if let Ok(uids) = session.uid_search(q).await { if let Some(uid) = uids.iter().copied().max() { uid_opt = Some(uid); break; } } } }
        if uid_opt.is_none() && account.provider.as_str() != "gmail" { for q in &fallback_variants { if let Ok(uids) = session.uid_search(q).await { if let Some(uid) = uids.iter().copied().max() { uid_opt = Some(uid); break; } } } }
        // If still none, fallback to existing retry loop below
        if uid_opt.is_some() { let _ = session.uid_store(&uid_opt.unwrap().to_string(), "+FLAGS (\\Seen)").await; }
        let _ = session.logout().await;
        return Ok(AppendResult { folder, uid: uid_opt });
    } else {
        // Skip APPEND for auto-sent providers; return expected Sent folder and uid if any
        let folder = candidates.first().cloned().unwrap_or_else(|| "Sent".to_string());
        let _ = session.logout().await;
        return Ok(AppendResult { folder, uid: uid_opt });
    }
}

/// Send email and log to events table (legacy). Prefer building and appending via append_to_sent.
pub async fn send_and_log(
    pool: &sqlx::SqlitePool,
    mailbox: &str,
    actor: Option<&str>,
    host: &str,
    username: &str,
    password: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<()> {
    // Send email
    // Varsayılan olarak 587 portu kullanılır, istenirse parametre eklenebilir
    send_simple(host, 587, username, password, to, subject, body)?;

    // Log OUT event
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    sqlx::query!(
        "INSERT INTO events (direction, mailbox, actor, peer, subject, ts) VALUES (?, ?, ?, ?, ?, ?)",
        "OUT",
        mailbox,
        actor,
        to,
        subject,
        ts
    )
    .execute(pool)
    .await?;

    tracing::info!(mailbox, to, subject, "Email sent and logged");
    Ok(())
}
