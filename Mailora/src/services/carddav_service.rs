/// CardDAV bidirectional sync service.
/// Implements: PROPFIND → address book discovery, REPORT → ETag diff,
/// GET new/changed → parse → DB upsert, PUT pending local changes, DELETE pending deletions.
use anyhow::{anyhow, Result};
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::contact::{AddressEntry, EmailEntry, PhoneEntry};
use crate::pim::dav_client::{DavClient, DavResource};
use crate::pim::vcard::{parse_vcard, serialize_vcard};
use crate::services::contact_service;

// ── Public entry point ────────────────────────────────────────

/// Full CardDAV sync for one account.
/// Called by scheduler (every 15 min) or manually via POST /sync/carddav/:account_id.
pub async fn sync_account(pool: &SqlitePool, account_id: &str) -> Result<SyncResult> {
    // 1. Load account credentials
    let account = load_account_creds(pool, account_id).await?;
    if account.carddav_url.is_none() {
        // No CardDAV URL — try auto-discovery first
        if let Some(url) = discover_carddav_url(&account).await {
            save_carddav_url(pool, account_id, &url).await?;
        } else {
            return Ok(SyncResult { synced: 0, created: 0, updated: 0, deleted: 0, conflicts: 0,
                message: "No CardDAV URL — discovery failed. Set manually.".into() });
        }
    }

    let carddav_url = account.carddav_url.as_deref().unwrap().to_string();
    let client = DavClient::new(&carddav_url, &account.username, &account.password);

    let mut total = SyncResult::default();

    // 2. Discover address books under the principal
    let addressbooks = list_addressbooks(&client, &carddav_url).await?;
    if addressbooks.is_empty() {
        return Ok(SyncResult { message: "No address books found".into(), ..Default::default() });
    }

    for ab in addressbooks {
        match sync_addressbook(pool, &client, account_id, &ab).await {
            Ok(r) => total.merge(r),
            Err(e) => warn!("CardDAV sync error for {}: {}", ab, e),
        }
    }

    info!("CardDAV sync complete for {}: {:?}", account_id, total);
    Ok(total)
}

// ── Address book sync ─────────────────────────────────────────

async fn sync_addressbook(
    pool: &SqlitePool,
    client: &DavClient,
    account_id: &str,
    ab_url: &str,
) -> Result<SyncResult> {
    let mut result = SyncResult::default();

    // Fetch remote {href → etag} map
    let remote_resources: Vec<DavResource> = match client.report_addressbook(ab_url).await {
        Ok(r) => r,
        Err(_) => client.propfind(ab_url, "1").await.unwrap_or_default(),
    };

    let remote_map: std::collections::HashMap<String, Option<String>> = remote_resources
        .iter()
        .map(|r| (r.href.clone(), r.etag.clone()))
        .collect();

    // Fetch local {href → etag} map for this addressbook
    let local_rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, href, etag FROM contacts WHERE account_id = ? AND href IS NOT NULL AND sync_status != 'pending_delete'"
    ).bind(account_id).fetch_all(pool).await?;

    let local_map: std::collections::HashMap<String, (String, Option<String>)> = local_rows
        .into_iter()
        .map(|(id, href, etag)| (href, (id, etag)))
        .collect();

    // ── DIFF: pull from remote ─────────────────────────────────

    for (href, remote_etag) in &remote_map {
        let local = local_map.get(href);
        let needs_fetch = match local {
            None => true,                              // New on server
            Some((_, local_etag)) => local_etag != remote_etag, // Changed on server
        };

        if !needs_fetch { continue; }

        match client.get(href).await {
            Ok((body, actual_etag)) => {
                let etag_to_use: Option<&str> = actual_etag.as_deref().or_else(|| remote_etag.as_deref());
                if let Some(local_entry) = local {
                    // Check for conflict: local also modified?
                    let local_status: Option<String> = sqlx::query_scalar(
                        "SELECT sync_status FROM contacts WHERE id = ?"
                    ).bind(&local_entry.0).fetch_optional(pool).await?;

                    if matches!(local_status.as_deref(), Some("pending_update")) {
                        // CONFLICT
                        record_conflict(pool, &local_entry.0, &body).await?;
                        result.conflicts += 1;
                        continue;
                    }

                    // Remote wins — update local
                    upsert_from_vcard(pool, account_id, &local_entry.0, &body, href, etag_to_use).await?;
                    result.updated += 1;
                } else {
                    // New contact from server
                    let new_id = Uuid::new_v4().to_string();
                    upsert_from_vcard(pool, account_id, &new_id, &body, href, etag_to_use).await?;
                    result.synced += 1;
                }
            }
            Err(e) => warn!("CardDAV GET failed for {}: {}", href, e),
        }
    }

    // ── Contacts deleted on server ─────────────────────────────

    for (href, (id, _)) in &local_map {
        if !remote_map.contains_key(href.as_str()) {
            // Server deleted this — remove locally unless pending_create (conflict case)
            let status: Option<String> = sqlx::query_scalar(
                "SELECT sync_status FROM contacts WHERE id = ?"
            ).bind(id).fetch_optional(pool).await?;
            if !matches!(status.as_deref(), Some("pending_create") | Some("pending_update")) {
                sqlx::query("DELETE FROM contacts WHERE id = ?").bind(id).execute(pool).await?;
                result.deleted += 1;
            }
        }
    }

    // ── PUSH: local pending_create ─────────────────────────────

    let pending_create: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM contacts WHERE account_id = ? AND sync_status = 'pending_create'"
    ).bind(account_id).fetch_all(pool).await?;

    for (id,) in pending_create {
        match push_create(pool, client, account_id, &id, ab_url).await {
            Ok(()) => result.created += 1,
            Err(e) => warn!("CardDAV push create failed for {}: {}", id, e),
        }
    }

    // ── PUSH: local pending_update ─────────────────────────────

    let pending_update: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, href, etag FROM contacts WHERE account_id = ? AND sync_status = 'pending_update'"
    ).bind(account_id).fetch_all(pool).await?;

    for (id, href, etag) in pending_update {
        if let Some(href) = href {
            match push_update(pool, client, &id, &href, etag.as_deref()).await {
                Ok(()) => result.updated += 1,
                Err(e) => {
                    if e.to_string().contains("CONFLICT") {
                        record_conflict(pool, &id, "ETag mismatch (remote changed)").await?;
                        result.conflicts += 1;
                    } else {
                        warn!("CardDAV push update failed for {}: {}", id, e);
                    }
                }
            }
        }
    }

    // ── PUSH: local pending_delete ─────────────────────────────

    let pending_delete: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, href, etag FROM contacts WHERE account_id = ? AND sync_status = 'pending_delete'"
    ).bind(account_id).fetch_all(pool).await?;

    for (id, href, etag) in pending_delete {
        if let Some(href) = href {
            match client.delete(&href, etag.as_deref()).await {
                Ok(()) => {
                    sqlx::query("DELETE FROM contacts WHERE id = ?").bind(&id).execute(pool).await?;
                    result.deleted += 1;
                }
                Err(e) => warn!("CardDAV DELETE failed for {}: {}", id, e),
            }
        } else {
            // No href = never reached server, just delete locally
            sqlx::query("DELETE FROM contacts WHERE id = ?").bind(&id).execute(pool).await?;
            result.deleted += 1;
        }
    }

    // Save sync state
    save_sync_state(pool, account_id, ab_url).await?;

    Ok(result)
}

// ── Push helpers ──────────────────────────────────────────────

async fn push_create(
    pool: &SqlitePool,
    client: &DavClient,
    account_id: &str,
    contact_id: &str,
    ab_url: &str,
) -> Result<()> {
    let vcard_str = build_vcard(pool, contact_id).await?;
    let uid = get_contact_uid(pool, contact_id).await?.unwrap_or_else(|| contact_id.to_string());
    let href = format!("{}/{}.vcf", ab_url.trim_end_matches('/'), uid);
    let new_etag = client.put(&href, &vcard_str, "text/vcard", None).await?;

    sqlx::query(
        "UPDATE contacts SET href = ?, etag = ?, vcard_uid = ?, sync_status = 'synced' WHERE id = ?"
    )
    .bind(&href)
    .bind(&new_etag)
    .bind(&uid)
    .bind(contact_id)
    .execute(pool).await?;
    Ok(())
}

async fn push_update(
    pool: &SqlitePool,
    client: &DavClient,
    contact_id: &str,
    href: &str,
    etag: Option<&str>,
) -> Result<()> {
    let vcard_str = build_vcard(pool, contact_id).await?;
    let new_etag = client.put(href, &vcard_str, "text/vcard", etag).await?;
    sqlx::query("UPDATE contacts SET etag = ?, sync_status = 'synced' WHERE id = ?")
        .bind(&new_etag).bind(contact_id)
        .execute(pool).await?;
    Ok(())
}

// ── Upsert from vCard ─────────────────────────────────────────

async fn upsert_from_vcard(
    pool: &SqlitePool,
    account_id: &str,
    contact_id: &str,
    raw: &str,
    href: &str,
    etag: Option<&str>,
) -> Result<()> {
    let Some(parsed) = parse_vcard(raw) else {
        warn!("Failed to parse vCard at {}", href);
        return Ok(());
    };
    let full_name = match &parsed.full_name {
        Some(n) => n.clone(),
        None => "Unknown".to_string(),
    };
    let now = chrono::Utc::now().to_rfc3339();

    // Check if contact already exists
    let exists: bool = sqlx::query_scalar("SELECT COUNT(1) FROM contacts WHERE id = ?")
        .bind(contact_id).fetch_one(pool).await.unwrap_or(0i64) > 0;

    if exists {
        sqlx::query(
            "UPDATE contacts SET full_name=?, first_name=?, last_name=?, middle_name=?, prefix=?, suffix=?, \
             company=?, department=?, title=?, note=?, birthday=?, photo_data=?, website_url=?, \
             vcard_uid=?, href=?, etag=?, raw_vcard=?, sync_status='synced', synced_at=?, updated_at=? \
             WHERE id=?"
        )
        .bind(&full_name).bind(&parsed.first_name).bind(&parsed.last_name)
        .bind(&parsed.middle_name).bind(&parsed.prefix).bind(&parsed.suffix)
        .bind(&parsed.company).bind(&parsed.department).bind(&parsed.title)
        .bind(&parsed.note).bind(&parsed.birthday).bind(&parsed.photo_data)
        .bind(&parsed.website_url).bind(&parsed.uid).bind(href).bind(etag)
        .bind(raw).bind(&now).bind(&now).bind(contact_id)
        .execute(pool).await?;

        // Refresh sub-records
        sqlx::query("DELETE FROM contact_emails WHERE contact_id = ?").bind(contact_id).execute(pool).await?;
        sqlx::query("DELETE FROM contact_phones WHERE contact_id = ?").bind(contact_id).execute(pool).await?;
        sqlx::query("DELETE FROM contact_addresses WHERE contact_id = ?").bind(contact_id).execute(pool).await?;
    } else {
        sqlx::query(
            "INSERT INTO contacts (id, account_id, full_name, first_name, last_name, middle_name, prefix, suffix, \
             company, department, title, note, birthday, photo_data, website_url, vcard_uid, href, etag, \
             raw_vcard, sync_status, synced_at, created_at, updated_at) \
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,'synced',?,?,?)"
        )
        .bind(contact_id).bind(account_id).bind(&full_name)
        .bind(&parsed.first_name).bind(&parsed.last_name)
        .bind(&parsed.middle_name).bind(&parsed.prefix).bind(&parsed.suffix)
        .bind(&parsed.company).bind(&parsed.department).bind(&parsed.title)
        .bind(&parsed.note).bind(&parsed.birthday).bind(&parsed.photo_data)
        .bind(&parsed.website_url).bind(&parsed.uid).bind(href).bind(etag)
        .bind(raw).bind(&now).bind(&now).bind(&now)
        .execute(pool).await?;
    }

    // Insert emails
    for e in &parsed.emails {
        let _ = sqlx::query("INSERT INTO contact_emails (id, contact_id, email, label, is_primary) VALUES (?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(contact_id)
            .bind(&e.email).bind(&e.label).bind(e.is_primary as i64)
            .execute(pool).await;
    }
    // Insert phones
    for p in &parsed.phones {
        let _ = sqlx::query("INSERT INTO contact_phones (id, contact_id, phone, label, is_primary) VALUES (?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(contact_id)
            .bind(&p.phone).bind(&p.label).bind(p.is_primary as i64)
            .execute(pool).await;
    }
    // Insert addresses
    for a in &parsed.addresses {
        let _ = sqlx::query("INSERT INTO contact_addresses (id, contact_id, label, street, city, region, postal_code, country) VALUES (?,?,?,?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(contact_id)
            .bind(&a.label).bind(&a.street).bind(&a.city)
            .bind(&a.region).bind(&a.postal_code).bind(&a.country)
            .execute(pool).await;
    }

    // Handle CATEGORIES → groups
    for cat in &parsed.categories {
        let gid: Option<String> = sqlx::query_scalar(
            "SELECT id FROM contact_groups WHERE account_id = ? AND name = ?"
        ).bind(account_id).bind(cat).fetch_optional(pool).await?;
        let gid = if let Some(g) = gid { g } else {
            contact_service::create_group(pool, account_id, cat, None).await?
        };
        let _ = sqlx::query("INSERT OR IGNORE INTO contact_group_members (contact_id, group_id) VALUES (?,?)")
            .bind(contact_id).bind(&gid).execute(pool).await;
    }

    Ok(())
}

// ── Auto-discovery (RFC 6764) ─────────────────────────────────

/// Try to discover the CardDAV base URL for an account.
pub async fn discover_carddav_url(account: &AccountCreds) -> Option<String> {
    let domain = account.email.split('@').nth(1)?;

    // 1. Hardcoded known providers
    let known = known_carddav_url(domain);
    if known.is_some() { return known; }

    // 2. .well-known/carddav
    let well_known = format!("https://{}/.well-known/carddav", domain);
    if probe_url(&well_known).await { return Some(well_known); }

    // 3. HTTP fallback
    let well_known_http = format!("http://{}/.well-known/carddav", domain);
    if probe_url(&well_known_http).await { return Some(well_known_http); }

    None
}

fn known_carddav_url(domain: &str) -> Option<String> {
    match domain {
        "gmail.com" | "googlemail.com" => Some("https://www.googleapis.com/carddav/v1/".into()),
        "icloud.com" | "me.com" | "mac.com" => Some("https://contacts.icloud.com/".into()),
        "fastmail.com" | "fastmail.fm" => Some("https://carddav.fastmail.com/".into()),
        "outlook.com" | "hotmail.com" | "live.com" => {
            // Microsoft has limited CardDAV support
            Some("https://contacts.office365.com/carddav/".into())
        }
        _ => None,
    }
}

async fn probe_url(url: &str) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
    else { return false; };

    match client.head(url).send().await {
        Ok(r) => r.status().is_success() || r.status().as_u16() == 401 || r.status().as_u16() == 207,
        Err(_) => false,
    }
}

// ── Address book listing ──────────────────────────────────────

async fn list_addressbooks(client: &DavClient, base_url: &str) -> Result<Vec<String>> {
    let mut addressbooks = Vec::new();

    // Try to get principal first
    let principal = client.discover_principal().await.unwrap_or_else(|_| "/".to_string());

    let resources = client.propfind(&principal, "1").await
        .unwrap_or_else(|_| Vec::new());

    for r in resources {
        // If it looks like a vcard collection (has .vcf or is listed without extension)
        if r.content_type.as_deref().map(|ct| ct.contains("vcard")).unwrap_or(false)
            || r.href.ends_with(".vcf") {
            continue; // Skip individual cards
        }
        // Heuristic: path contains "addressbook" or "contacts"
        let lower = r.href.to_lowercase();
        if lower.contains("addressbook") || lower.contains("contacts") || lower.contains("carddav") {
            addressbooks.push(r.href);
        }
    }

    // If nothing found, use base URL itself as the address book
    if addressbooks.is_empty() {
        addressbooks.push(base_url.to_string());
    }

    Ok(addressbooks)
}

// ── Build vCard from DB ───────────────────────────────────────

async fn build_vcard(pool: &SqlitePool, contact_id: &str) -> Result<String> {
    let Some(full) = contact_service::get_contact(pool, contact_id).await? else {
        return Err(anyhow!("Contact not found: {}", contact_id));
    };
    let c = &full.contact;
    let uid = c.vcard_uid.as_deref().unwrap_or(&c.id);
    let emails: Vec<EmailEntry> = full.emails.iter().map(|e| EmailEntry {
        email: e.email.clone(), label: e.label.clone(), is_primary: e.is_primary != 0
    }).collect();
    let phones: Vec<PhoneEntry> = full.phones.iter().map(|p| PhoneEntry {
        phone: p.phone.clone(), label: p.label.clone(), is_primary: p.is_primary != 0
    }).collect();
    let addresses: Vec<AddressEntry> = full.addresses.iter().map(|a| AddressEntry {
        label: a.label.clone(), street: a.street.clone(), city: a.city.clone(),
        region: a.region.clone(), postal_code: a.postal_code.clone(), country: a.country.clone()
    }).collect();

    Ok(serialize_vcard(
        uid, &c.full_name,
        c.first_name.as_deref(), c.last_name.as_deref(), c.middle_name.as_deref(),
        c.prefix.as_deref(), c.suffix.as_deref(),
        c.company.as_deref(), c.department.as_deref(), c.title.as_deref(),
        c.note.as_deref(), c.birthday.as_deref(),
        &emails, &phones, &addresses, &[],
    ))
}

async fn get_contact_uid(pool: &SqlitePool, contact_id: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT COALESCE(vcard_uid, id) FROM contacts WHERE id = ?")
        .bind(contact_id).fetch_optional(pool).await?)
}

// ── Conflict recording ────────────────────────────────────────

async fn record_conflict(pool: &SqlitePool, contact_id: &str, remote_data: &str) -> Result<()> {
    // Read local data snapshot
    let local_data: String = sqlx::query_scalar("SELECT raw_vcard FROM contacts WHERE id = ?")
        .bind(contact_id).fetch_optional(pool).await?
        .flatten()
        .unwrap_or_else(|| "{}".into());

    sqlx::query(
        "INSERT INTO contact_conflicts (id, contact_id, local_data, remote_data) VALUES (?,?,?,?)"
    )
    .bind(Uuid::new_v4().to_string())
    .bind(contact_id)
    .bind(&local_data)
    .bind(remote_data)
    .execute(pool).await?;

    sqlx::query("UPDATE contacts SET sync_status = 'conflict' WHERE id = ?")
        .bind(contact_id).execute(pool).await?;

    Ok(())
}

// ── Sync state persistence ────────────────────────────────────

async fn save_sync_state(pool: &SqlitePool, account_id: &str, ab_url: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO carddav_sync_state (account_id, addressbook_url, last_synced_at) \
         VALUES (?,?,?) ON CONFLICT(account_id, addressbook_url) DO UPDATE SET last_synced_at=excluded.last_synced_at"
    ).bind(account_id).bind(ab_url).bind(&now)
    .execute(pool).await?;
    Ok(())
}

async fn save_carddav_url(pool: &SqlitePool, account_id: &str, url: &str) -> Result<()> {
    sqlx::query("UPDATE accounts SET carddav_url = ? WHERE id = ?")
        .bind(url).bind(account_id).execute(pool).await?;
    Ok(())
}

// ── Account credentials ───────────────────────────────────────

pub struct AccountCreds {
    pub id: String,
    pub email: String,
    pub username: String,
    pub password: String,
    pub carddav_url: Option<String>,
}

async fn load_account_creds(pool: &SqlitePool, account_id: &str) -> Result<AccountCreds> {
    #[derive(sqlx::FromRow)]
    struct RawRow {
        id: String,
        email: String,
        credentials_encrypted: String,
        carddav_url: Option<String>,
    }
    let row: RawRow = sqlx::query_as(
        "SELECT id, email, credentials_encrypted, carddav_url FROM accounts WHERE id = ?"
    ).bind(account_id).fetch_optional(pool).await?
     .ok_or_else(|| anyhow!("Account not found"))?;

    let (_, password) = crate::models::account::Account::decode_credentials(&row.credentials_encrypted)
        .map_err(|e| anyhow!("Failed to decode credentials: {}", e))?;

    Ok(AccountCreds {
        id: row.id,
        username: row.email.clone(),
        email: row.email,
        password,
        carddav_url: row.carddav_url,
    })
}

// ── Sync result ───────────────────────────────────────────────

#[derive(Debug, Default, serde::Serialize)]
pub struct SyncResult {
    pub synced: usize,    // New from server
    pub created: usize,   // Pushed to server
    pub updated: usize,   // Updated (both directions)
    pub deleted: usize,
    pub conflicts: usize,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
}

impl SyncResult {
    fn merge(&mut self, other: SyncResult) {
        self.synced += other.synced;
        self.created += other.created;
        self.updated += other.updated;
        self.deleted += other.deleted;
        self.conflicts += other.conflicts;
    }
}
