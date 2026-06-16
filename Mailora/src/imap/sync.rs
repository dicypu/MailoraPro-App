use anyhow::Result;
use async_imap::Session;
// use base64::Engine; // needed for STANDARD.decode
use futures::StreamExt;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_native_tls::native_tls::TlsConnector;
use tokio_util::compat::TokioAsyncReadCompatExt;
use mail_parser::MimeHeaders;

#[derive(Debug, Serialize)]
pub struct NewMessageMeta {
    pub uid: u32,
    pub subject: String,
    pub from: String,
    pub date: Option<String>,
    pub size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct MessageBodyMeta {
    pub uid: u32,
    pub subject: String,
    pub from: String,
    pub date: Option<String>,
    pub size: Option<u32>,
    pub flags: Vec<String>,
    pub body: String,
    pub html_body: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AttachmentMeta {
    pub uid: u32,
    pub part_id: String,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct AttachmentSummary { pub count: usize, pub top: Vec<String> }

pub struct SnapshotResult {
    pub uidvalidity: u32,
    pub last_uid: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct MessageState {
    pub uid: u32,
    pub flags: Vec<String>,
}

pub async fn initial_snapshot(
    host: &str,
    port: u16,
    email: &str,
    password: &str,
) -> Result<SnapshotResult> {
    if port == 143 {
        let tcp = TcpStream::connect((host, port)).await?;
        let client = async_imap::Client::new(tcp.compat());
        let mut session: Session<_> = client
            .login(email, password)
            .await
            .map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
        let mailbox = session.select("INBOX").await?;
        let mut last_uid: u32 = 0;
        if let Ok(uids) = session.uid_search("ALL").await {
            for uid in uids {
                if uid > last_uid {
                    last_uid = uid;
                }
            }
        }
        let _ = session.logout().await;
        return Ok(SnapshotResult {
            uidvalidity: mailbox.uid_validity.unwrap_or(0) as u32,
            last_uid,
        });
    } else {
        let tcp = TcpStream::connect((host, port)).await?;
        let tls = TlsConnector::builder().build()?;
        let tls = tokio_native_tls::TlsConnector::from(tls);
        let tls_stream = tls.connect(host, tcp).await?;
        let client = async_imap::Client::new(tls_stream.compat());
        let mut session: Session<_> = client
            .login(email, password)
            .await
            .map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
        let mailbox = session.select("INBOX").await?;
        let uidvalidity = mailbox.uid_validity.unwrap_or(0) as u32;
        let mut last_uid: u32 = 0;
        if let Ok(uids) = session.uid_search("ALL").await {
            for uid in uids {
                if uid > last_uid {
                    last_uid = uid;
                }
            }
        }
        let _ = session.logout().await;
        return Ok(SnapshotResult {
            uidvalidity,
            last_uid,
        });
    }
}

pub async fn fetch_new_since(
    host: &str,
    port: u16,
    email: &str,
    password: &str,
    last_uid: u32,
) -> Result<(u32, Vec<NewMessageMeta>)> {
    if port == 143 {
        let tcp = TcpStream::connect((host, port)).await?;
        let client = async_imap::Client::new(tcp.compat());
        let mut session: Session<_> = client
            .login(email, password)
            .await
            .map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
        let mailbox = session.select("INBOX").await?;
        let all = session.uid_search("ALL").await?;
        let mut present_max: u32 = last_uid;
        for uid in &all {
            if *uid > present_max {
                present_max = *uid;
            }
        }
        let mut newer: Vec<u32> = all
            .into_iter()
            .filter(|u| *u >= last_uid.saturating_sub(10))
            .collect();
        newer.sort_unstable();
        tracing::debug!(
            last_uid,
            uid_next = mailbox.uid_next.map(|v| v as u32),
            present_max,
            newer_count = newer.len(),
            "imap.fetch_new_since using ALL to compute newer UIDs"
        );
        if newer.is_empty() {
            let _ = session.logout().await;
            return Ok((last_uid, Vec::new()));
        }
        let new_last = *newer.last().unwrap_or(&present_max);
        let seq = newer
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");
        tracing::debug!(%seq, count = newer.len(), "imap.fetch_new_since fetching exact newer UIDs");
        let mut out = Vec::new();
        let mut fetches = session
            .uid_fetch(&seq, "UID ENVELOPE FLAGS INTERNALDATE")
            .await?;
        while let Some(item) = fetches.next().await {
            let f = item?;
            if let Some(uid) = f.uid {
                if uid <= last_uid {
                    continue;
                }
                let env = f.envelope();
                let subject = env
                    .and_then(|e| e.subject.as_ref())
                    .map(|b| decode_subject(b))
                    .unwrap_or_default();
                let from = env
                    .and_then(|e| e.from.as_ref())
                    .and_then(|v| v.get(0))
                    .map(format_address)
                    .unwrap_or_default();
                let date = f.internal_date().map(|d| d.to_rfc3339());
                let size = None;
                out.push(NewMessageMeta {
                    uid,
                    subject,
                    from,
                    date,
                    size,
                });
            }
        }
        drop(fetches);
        tracing::debug!(
            fetched = out.len(),
            new_last,
            "imap.fetch_new_since completed"
        );
        let _ = session.logout().await;
        return Ok((new_last, out));
    } else {
        let tcp = TcpStream::connect((host, port)).await?;
        let tls = TlsConnector::builder().build()?;
        let tls = tokio_native_tls::TlsConnector::from(tls);
        let tls_stream = tls.connect(host, tcp).await?;
        let client = async_imap::Client::new(tls_stream.compat());
        let mut session: Session<_> = client
            .login(email, password)
            .await
            .map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
        let mailbox = session.select("INBOX").await?;
        let all = session.uid_search("ALL").await?;
        let mut present_max: u32 = last_uid;
        for uid in &all {
            if *uid > present_max {
                present_max = *uid;
            }
        }
        let mut newer: Vec<u32> = all
            .into_iter()
            .filter(|u| *u >= last_uid.saturating_sub(10))
            .collect();
        newer.sort_unstable();
        tracing::debug!(
            last_uid,
            uid_next = mailbox.uid_next.map(|v| v as u32),
            present_max,
            newer_count = newer.len(),
            "imap.fetch_new_since using ALL to compute newer UIDs"
        );
        if newer.is_empty() {
            let _ = session.logout().await;
            return Ok((last_uid, Vec::new()));
        }
        let new_last = *newer.last().unwrap_or(&present_max);
        let seq = newer
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");
        tracing::debug!(%seq, count = newer.len(), "imap.fetch_new_since fetching exact newer UIDs");
        let mut out = Vec::new();
        let mut fetches = session
            .uid_fetch(&seq, "UID ENVELOPE FLAGS INTERNALDATE")
            .await?;
        while let Some(item) = fetches.next().await {
            let f = item?;
            if let Some(uid) = f.uid {
                if uid <= last_uid {
                    continue;
                }
                let env = f.envelope();
                let subject = env
                    .and_then(|e| e.subject.as_ref())
                    .map(|b| decode_subject(b))
                    .unwrap_or_default();
                let from = env
                    .and_then(|e| e.from.as_ref())
                    .and_then(|v| v.get(0))
                    .map(format_address)
                    .unwrap_or_default();
                let date = f.internal_date().map(|d| d.to_rfc3339());
                let size = None;
                out.push(NewMessageMeta {
                    uid,
                    subject,
                    from,
                    date,
                    size,
                });
            }
        }
        drop(fetches);
        tracing::debug!(
            fetched = out.len(),
            new_last,
            "imap.fetch_new_since completed"
        );
        let _ = session.logout().await;
        return Ok((new_last, out));
    }
}

pub async fn fetch_message_body(
    host: &str,
    port: u16,
    email: &str,
    password: &str,
    uid: u32,
) -> Result<Option<MessageBodyMeta>> {
    fetch_message_body_in(host, port, email, password, uid, "INBOX").await
}

pub async fn fetch_message_body_in(
    host: &str,
    port: u16,
    email: &str,
    password: &str,
    uid: u32,
    folder: &str,
) -> Result<Option<MessageBodyMeta>> {
    if port == 143 {
        let tcp = TcpStream::connect((host, port)).await?;
        let client = async_imap::Client::new(tcp.compat());
        let mut session: Session<_> = client
            .login(email, password)
            .await
            .map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
        tracing::debug!(%folder, uid, "body_in: selecting folder");
        session.select(folder).await?;
        let _ = session.noop().await;
        let uid_str = uid.to_string();
        let mut head = session
            .uid_fetch(&uid_str, "UID ENVELOPE FLAGS BODYSTRUCTURE")
            .await?;
        let mut base: Option<(String, String, Option<String>, Vec<String>)> = None;
        let mut head_count: usize = 0;
        let mut head_uids: Vec<Option<u32>> = Vec::new();
        while let Some(item) = head.next().await {
            let f = item?;
            head_count += 1;
            head_uids.push(f.uid);
            if let Some(f_uid) = f.uid {
                if f_uid != uid {
                    continue;
                }
            }
            let env = f.envelope();
            let subject = env
                .and_then(|e| e.subject.as_ref())
                .map(|b| decode_subject(b))
                .unwrap_or_default();
            let from = env
                .and_then(|e| e.from.as_ref())
                .and_then(|v| v.get(0))
                .map(format_address)
                .unwrap_or_default();
            let date = f.internal_date().map(|d| d.to_rfc3339());
            let flags: Vec<String> = f.flags().map(|fl| format!("{:?}", fl)).collect();
            base = Some((subject, from, date, flags));
        }
        tracing::debug!(%folder, uid, head_count, head_uids=?head_uids, "body_in: initial meta fetch result");
        drop(head);
        // Don't return early when base is missing; we'll try raw BODY[] and parse headers
        let (mut subject, mut from, mut date, flags) = match base {
            Some((s,f,d,fl)) => (s,f,d,fl),
            None => (String::new(), String::new(), None, Vec::new()),
        };
        let candidates = [
            "BODY.PEEK[]", // full raw message first to allow MIME parsing
            "BODY.PEEK[TEXT]",
            "BODY.PEEK[1.TEXT]",
            "BODY.PEEK[1.1.TEXT]",
            "BODY.PEEK[1]",
            "BODY.PEEK[1.1]",
        ];
        let mut body: Option<Vec<u8>> = None;
        let mut raw_full: Option<Vec<u8>> = None;
        let mut chosen: &str = "";
        for sect in &candidates {
            tracing::debug!(%folder, uid, section=%sect, "body_in: trying section");
            let mut part = session.uid_fetch(&uid_str, sect).await?;
            let mut got = None;
            while let Some(item) = part.next().await {
                let f = item?;
                if let Some(b) = f.body() {
                    if !b.is_empty() {
                        got = Some(b.to_vec());
                        break;
                    }
                }
            }
            drop(part);
            if let Some(v) = got {
                if *sect == "BODY.PEEK[]" { raw_full = Some(v.clone()); }
                if body.is_none() { chosen = sect; body = Some(v); }
                if body.is_some() && raw_full.is_some() { break; }
            }
        }
        // If ENVELOPE missing, try header fields from raw
        if subject.is_empty() || from.is_empty() || date.is_none() {
            if let Some(_full) = raw_full.as_ref() {

            }
        }
        let mut body_text = String::new();
        let mut html_opt: Option<String> = None;
        if let Some(bytes) = body {
            let mut s = String::from_utf8(bytes.clone())
                .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).to_string());
            if s.len() > 8000 { s.truncate(8000); s.push_str("\n...[truncated]..."); }
            tracing::debug!(%folder, uid, section=%chosen, len=s.len(), "body_in: got body");
            body_text = s;
        }
        // MIME parse for Header Fallback + HTML/Text Body
        if let Some(full) = raw_full {
            if full.len() <= 50_000_000 { // safety cap 50MB
                 if let Some(message) = mail_parser::Message::parse(&full) {
                     // 1. Header Fallback
                     if subject.is_empty() { subject = message.subject().unwrap_or("").to_string(); }
                     if from.is_empty() { 
                         match message.from() {
                             mail_parser::HeaderValue::Address(addr) => {
                                 from = addr.address.as_deref().unwrap_or("").to_string();
                             }
                             mail_parser::HeaderValue::AddressList(list) => {
                                 if let Some(first) = list.first() {
                                     from = first.address.as_deref().unwrap_or("").to_string();
                                 }
                             }
                             _ => {}
                         }
                     }
                     if date.is_none() { 
                         if let Some(dt) = message.date() {
                             date = Some(dt.to_rfc3339());
                         }
                     }

                     // 2. Body Extraction
                     if let Some(text) = message.body_text(0) {
                         body_text = text.into_owned();
                     }
                     if let Some(html) = message.body_html(0) {
                         let mut h2 = html.into_owned();
                         // Safety truncate logic
                         if h2.len() > 5_000_000 { h2.truncate(5_000_000); h2.push_str("\n...[html truncated]..."); }
                         html_opt = Some(h2);
                     }
                 }
            }
        }
        let _ = session.logout().await;
        return Ok(Some(MessageBodyMeta {
            uid,
            subject,
            from,
            date,
            size: None,
            flags,
            body: body_text,
            html_body: html_opt,
        }));
    } else {
        let tcp = TcpStream::connect((host, port)).await?;
        let tls = TlsConnector::builder().build()?;
        let tls = tokio_native_tls::TlsConnector::from(tls);
        let tls_stream = tls.connect(host, tcp).await?;
        let client = async_imap::Client::new(tls_stream.compat());
        let mut session: Session<_> = client
            .login(email, password)
            .await
            .map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
        tracing::debug!(%folder, uid, "body_in: selecting folder");
        session.select(folder).await?;
        let _ = session.noop().await;
        let uid_str = uid.to_string();
        let mut head = session
            .uid_fetch(&uid_str, "UID ENVELOPE FLAGS BODYSTRUCTURE")
            .await?;
        let mut base: Option<(String, String, Option<String>, Vec<String>)> = None;
        let mut head_count: usize = 0;
        let mut head_uids: Vec<Option<u32>> = Vec::new();
        while let Some(item) = head.next().await {
            let f = item?;
            head_count += 1;
            head_uids.push(f.uid);
            if let Some(f_uid) = f.uid {
                if f_uid != uid {
                    continue;
                }
            }
            let env = f.envelope();
            let subject = env
                .and_then(|e| e.subject.as_ref())
                .map(|b| decode_subject(b))
                .unwrap_or_default();
            let from = env
                .and_then(|e| e.from.as_ref())
                .and_then(|v| v.get(0))
                .map(format_address)
                .unwrap_or_default();
            let date = f.internal_date().map(|d| d.to_rfc3339());
            let flags: Vec<String> = f.flags().map(|fl| format!("{:?}", fl)).collect();
            base = Some((subject, from, date, flags));
        }
        tracing::debug!(%folder, uid, head_count, head_uids=?head_uids, "body_in: initial meta fetch result");
        drop(head);
        // Don't return early when base is missing; we'll try raw BODY[] and parse headers
        let (mut subject, mut from, mut date, flags) = match base {
            Some((s,f,d,fl)) => (s,f,d,fl),
            None => (String::new(), String::new(), None, Vec::new()),
        };
        let candidates = [
            "BODY.PEEK[]", // full raw message first to allow MIME parsing
            "BODY.PEEK[TEXT]",
            "BODY.PEEK[1.TEXT]",
            "BODY.PEEK[1.1.TEXT]",
            "BODY.PEEK[1]",
            "BODY.PEEK[1.1]",
        ];
        let mut body: Option<Vec<u8>> = None;
        let mut raw_full: Option<Vec<u8>> = None;
        let mut chosen: &str = "";
        for sect in &candidates {
            tracing::debug!(%folder, uid, section=%sect, "body_in: trying section");
            let mut part = session.uid_fetch(&uid_str, sect).await?;
            let mut got = None;
            while let Some(item) = part.next().await {
                let f = item?;
                if let Some(b) = f.body() {
                    if !b.is_empty() {
                        got = Some(b.to_vec());
                        break;
                    }
                }
            }
            drop(part);
            if let Some(v) = got {
                if *sect == "BODY.PEEK[]" { raw_full = Some(v.clone()); }
                if body.is_none() { chosen = sect; body = Some(v); }
                if body.is_some() && raw_full.is_some() { break; }
            }
        }
        // If ENVELOPE missing, try header fields from raw
        if subject.is_empty() || from.is_empty() || date.is_none() {
            if let Some(_full) = raw_full.as_ref() {

            }
        }
        let mut body_text = String::new();
        let mut html_opt: Option<String> = None;
        if let Some(bytes) = body {
            let mut s = String::from_utf8(bytes.clone())
                .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).to_string());
            if s.len() > 8000 { s.truncate(8000); s.push_str("\n...[truncated]..."); }
            tracing::debug!(%folder, uid, section=%chosen, len=s.len(), "body_in: got body");
            body_text = s;
        }
        // MIME parse for Header Fallback + HTML/Text Body
        if let Some(full) = raw_full {
            if full.len() <= 50_000_000 { // safety cap 50MB
                 if let Some(message) = mail_parser::Message::parse(&full) {
                     // 1. Header Fallback
                     if subject.is_empty() { subject = message.subject().unwrap_or("").to_string(); }
                     if from.is_empty() { 
                         match message.from() {
                             mail_parser::HeaderValue::Address(addr) => {
                                 from = addr.address.as_deref().unwrap_or("").to_string();
                             }
                             mail_parser::HeaderValue::AddressList(list) => {
                                 if let Some(first) = list.first() {
                                     from = first.address.as_deref().unwrap_or("").to_string();
                                 }
                             }
                             _ => {}
                         }
                     }
                     if date.is_none() { 
                         if let Some(dt) = message.date() {
                             date = Some(dt.to_rfc3339());
                         }
                     }

                     // 2. Body Extraction
                     if let Some(text) = message.body_text(0) {
                         body_text = text.into_owned();
                     }
                     if let Some(html) = message.body_html(0) {
                         let mut h2 = html.into_owned();
                         // Safety truncate logic
                         if h2.len() > 5_000_000 { h2.truncate(5_000_000); h2.push_str("\n...[html truncated]..."); }
                         html_opt = Some(h2);
                     }
                 }
            }
        }
        let _ = session.logout().await;
        return Ok(Some(MessageBodyMeta {
            uid,
            subject,
            from,
            date,
            size: None,
            flags,
            body: body_text,
            html_body: html_opt,
        }));
    }
}

fn format_address(a: &async_imap::imap_proto::Address<'_>) -> String {
    let name = a.name.as_ref().map(|n| decode_subject(n)).unwrap_or_default();
    let mailbox = a.mailbox.as_ref().map(|b| String::from_utf8_lossy(b).to_string()).unwrap_or_default();
    let host = a.host.as_ref().map(|b| String::from_utf8_lossy(b).to_string()).unwrap_or_default();
    let mut s = String::new();
    if !name.is_empty() { s.push_str(&name); s.push(' '); }
    if !mailbox.is_empty() || !host.is_empty() { s.push('<'); s.push_str(&mailbox); if !host.is_empty(){ s.push('@'); s.push_str(&host);} s.push('>'); }
    s.trim().to_string()
}

pub(crate) fn decode_subject(raw: &[u8]) -> String {
    let mut composed = b"Subject: ".to_vec(); composed.extend_from_slice(raw); composed.extend_from_slice(b"\r\n\r\n");
    if let Some(msg) = mail_parser::Message::parse(&composed) {
        msg.subject().unwrap_or("").to_string()
    } else {
        String::from_utf8_lossy(raw).trim().to_string()
    }
}

pub async fn list_attachments(
    host: &str,
    port: u16,
    email: &str,
    password: &str,
    folder: &str,
    uid: u32,
) -> Result<Vec<AttachmentMeta>> {
    use futures::StreamExt;
    let tcp = TcpStream::connect((host, port)).await?;
    let tls = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let tls = tokio_native_tls::TlsConnector::from(tls);
    let tls_stream = tls.connect(host, tcp).await?;
    let client = async_imap::Client::new(tls_stream.compat());
    let mut session: Session<_> = client.login(email, password).await.map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
    session.select(folder).await?;
    let uid_str = uid.to_string();
    let mut fetches = session.uid_fetch(&uid_str, "UID BODY.PEEK[]").await?;
    let mut raw: Option<Vec<u8>> = None;
    while let Some(item) = fetches.next().await {
        let f = item?;
        if f.uid == Some(uid) {
            if let Some(b) = f.body() { raw = Some(b.to_vec()); break; }
        }
    }
    drop(fetches);
    let _ = session.logout().await;
    let mut out = Vec::new();
    let raw = match raw { Some(r) => r, None => return Ok(out) };
    // Increase cap to handle larger emails with attachments (50MB)
    if raw.len() > 50_000_000 {
        tracing::debug!(uid, folder=%folder, size=raw.len(), "list_attachments: raw too large, skipping parse");
        return Ok(out);
    }

    // Refactored to use mail-parser for robust MIME handling
    if let Some(message) = mail_parser::Message::parse(&raw) {
        tracing::info!(uid, folder=%folder, "list_attachments: parsed successfully");
        let mut idx = 1;
        // Use parts iteration
        // Use parts iteration
        for (part_idx, part) in message.parts.iter().enumerate() {
            let ctype = part.content_type();
            let c_type = ctype.map(|c| c.c_type.as_ref()).unwrap_or("application");
            let subtype = ctype.and_then(|c| c.subtype()).unwrap_or("");
            
            // Skip structural parts
            if c_type == "multipart" { 
                tracing::debug!(uid, part_idx, "list_attachments: skipping multipart container");
                continue; 
            }

            let is_body = c_type == "text" && (subtype == "plain" || subtype == "html");
            let has_filename = part.attachment_name().is_some();
            let has_cid = part.content_id().is_some();
            let is_inline = part.content_disposition().map(|cd| cd.c_type.eq_ignore_ascii_case("inline")).unwrap_or(false);
            
            let ctype_full = format!("{}/{}", c_type, subtype);
            let is_media = c_type == "image" || c_type == "video" || c_type == "audio" || c_type == "application";

            tracing::info!(
                uid, part_idx, 
                is_body, has_filename, has_cid, is_inline, ?ctype_full, 
                "list_attachments: inspecting part"
            );

            // Logic:
            // 1. If it has a filename, it's an attachment (likely).
            // 2. If it's media (image/pdf/etc) and NOT explicitly marked as a purely structure body (unlikely for application/pdf), include it.
            // 3. Explicitly exclude text/plain and text/html bodies unless they have a filename.
            if has_filename || (is_media && !is_body) {
                 let fname = part.attachment_name().map(|s| s.to_string()).unwrap_or_else(|| {
                     let ext = match subtype {
                         "jpeg" => "jpg",
                         "png" => "png",
                         "gif" => "gif",
                         "pdf" => "pdf",
                         _ => "bin"
                     };
                     format!("unnamed_{}.{}", idx, ext)
                 });

                 out.push(AttachmentMeta {
                    uid,
                    part_id: format!("{}", idx), 
                    filename: Some(fname),
                    content_type: Some(ctype_full),
                    size: Some(part.contents().len() as u64),
                 });
                 idx += 1;
            }

        }
    } else {
        tracing::warn!(uid, folder=%folder, "list_attachments: parse failed");
    }
    Ok(out)
}

pub async fn fetch_attachment_part(
    host: &str,
    port: u16,
    email: &str,
    password: &str,
    folder: &str,
    uid: u32,
    target_part: &str,
) -> Result<Option<(Vec<u8>, Option<String>, Option<String>)>> {
    use futures::StreamExt;
    let tcp = TcpStream::connect((host, port)).await?;
    let tls = TlsConnector::builder().build()?;
    let tls = tokio_native_tls::TlsConnector::from(tls);
    let tls_stream = tls.connect(host, tcp).await?;
    let client = async_imap::Client::new(tls_stream.compat());
    let mut session: Session<_> = client.login(email, password).await.map_err(|e| anyhow::anyhow!("login failed: {:?}", e))?;
    session.select(folder).await?;
    let uid_str = uid.to_string();
    let mut fetches = session.uid_fetch(&uid_str, "UID BODY.PEEK[]").await?;
    let mut raw: Option<Vec<u8>> = None;
    while let Some(item) = fetches.next().await { let f = item?; if f.uid == Some(uid) { if let Some(b) = f.body() { raw = Some(b.to_vec()); break; } } }
    drop(fetches);
    let _ = session.logout().await;
    let raw = match raw { Some(r) => r, None => return Ok(None) };
    // Increase cap here as well
    if raw.len() > 50_000_000 { return Ok(None); }
    
    if let Some(message) = mail_parser::Message::parse(&raw) {
        // Mode 1: Numeric index (from list_attachments)
        if let Ok(idx) = target_part.parse::<usize>() {
            let mut current = 0;
            for attachment in message.attachments() {
                current += 1;
                if current == idx {
                    let ctype = attachment.content_type().map(|c| format!("{}/{}", c.c_type, c.subtype().unwrap_or(""))).unwrap_or("application/octet-stream".into());
                    return Ok(Some((attachment.contents().to_vec(), Some(ctype), attachment.attachment_name().map(|s| s.to_string()))));
                }
            }
        }

        // Mode 2: Content-ID (for inline images)
        let wanted = target_part.trim().trim_matches(['<','>']).to_lowercase();
        for attachment in message.attachments() {
             if let Some(cid) = attachment.content_id() {
                 if cid.trim().trim_matches(['<','>']).to_lowercase() == wanted {
                    let ctype = attachment.content_type().map(|c| format!("{}/{}", c.c_type, c.subtype().unwrap_or(""))).unwrap_or("application/octet-stream".into());
                    return Ok(Some((attachment.contents().to_vec(), Some(ctype), attachment.attachment_name().map(|s| s.to_string()))));
                 }
             }
        }
    }
    Ok(None)
}

fn percent_decode_simple(s: &str) -> String {
    // Helpers removed (extract_filename_from_headers, decode_rfc2231, etc.) as we now use mail-parser.
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h1 = bytes[i + 1];
            let h2 = bytes[i + 2];
            let v = (match (h1 as char).to_digit(16) { Some(v) => v, None => { out.push(bytes[i]); i += 1; continue; } } << 4)
                | match (h2 as char).to_digit(16) { Some(v) => v, None => { out.push(bytes[i]); i += 1; continue; } };
            out.push(v as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).to_string()
}
