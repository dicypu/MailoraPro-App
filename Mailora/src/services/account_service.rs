/// Account management service
use crate::models::account::{Account, EmailProvider};
use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::Row;

/// Add a new email account
pub async fn add_account(
    pool: &SqlitePool,
    email: &str,
    password: &str,
    provider: EmailProvider,
    display_name: Option<String>,
    custom_config: Option<(String, u16, String, u16)>, // (imap_host, imap_port, smtp_host, smtp_port)
) -> Result<Account> {
    let id = Account::generate_id(email);

    // Check if account already exists
    let existing = sqlx::query!("SELECT id FROM accounts WHERE id = ?", id)
        .fetch_optional(pool)
        .await?;

    if existing.is_some() {
        anyhow::bail!("Account already exists: {}", email);
    }

    // Get provider config
    let config = provider.default_config();
    let (imap_host, imap_port, smtp_host, smtp_port) = if let Some((ih, ip, sh, sp)) = custom_config
    {
        (ih, ip, sh, sp)
    } else {
        (
            config.imap_host,
            config.imap_port,
            config.smtp_host,
            config.smtp_port,
        )
    };

    let computed_carddav_url = config.carddav_url.map(|u| u.replace("[EMAIL]", email));
    let computed_caldav_url = config.caldav_url.map(|u| u.replace("[EMAIL]", email));

    let credentials_encrypted = Account::encode_credentials(email, password);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    let provider_str = provider.as_str();
    let display_name_str = display_name.as_deref();
    let enabled = true;
    let sync_freq: i64 = 300;
    let initial_last_uid: u32 = 0; // Fetch all emails on first sync

    sqlx::query(
        r#"
        INSERT INTO accounts (
            id, email, provider, display_name, 
            imap_host, imap_port, smtp_host, smtp_port,
            credentials_encrypted, enabled, sync_frequency_secs,
            created_at, updated_at, color, carddav_url, caldav_url
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(email)
    .bind(provider_str)
    .bind(display_name_str)
    .bind(&imap_host)
    .bind(imap_port)
    .bind(&smtp_host)
    .bind(smtp_port)
    .bind(&credentials_encrypted)
    .bind(enabled)
    .bind(sync_freq)
    .bind(now)
    .bind(now)
    .bind("#3b82f6")
    .bind(&computed_carddav_url)
    .bind(&computed_caldav_url)
    .execute(pool)
    .await?;

    let account = Account {
        id,
        email: email.to_string(),
        provider,
        display_name,
        imap_host,
        imap_port,
        smtp_host,
        smtp_port,
        credentials_encrypted,
        enabled: true,
        sync_frequency_secs: 300,
        last_sync_ts: Some(initial_last_uid as i64),
        created_at: now,
        updated_at: now,
        append_policy: Some("auto".to_string()),
        sent_folder_hint: None,
        color: Some("#3b82f6".to_string()),
        carddav_url: computed_carddav_url,
        caldav_url: computed_caldav_url,
        password: String::new(),
    };

    Ok(account)
}

/// Get all accounts
pub async fn list_accounts(pool: &SqlitePool) -> Result<Vec<Account>> {
    let rows = sqlx::query(
        r#"SELECT * FROM accounts ORDER BY created_at DESC"#
    )
    .fetch_all(pool)
    .await?;

    let mut accounts = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.try_get("id").unwrap_or_default();
        let email: String = row.try_get("email").unwrap_or_default();
        let provider_s: String = row.try_get("provider").unwrap_or_else(|_| "custom".to_string());
        let display_name: Option<String> = row.try_get("display_name").ok();
        let imap_host: String = row.try_get("imap_host").unwrap_or_default();
        let imap_port_i: i64 = row.try_get("imap_port").unwrap_or(993);
        let smtp_host: String = row.try_get("smtp_host").unwrap_or_default();
        let smtp_port_i: i64 = row.try_get("smtp_port").unwrap_or(587);
        let credentials_encrypted: String = row.try_get("credentials_encrypted").unwrap_or_default();
        let enabled_i: i64 = row.try_get("enabled").unwrap_or(1);
        let sync_frequency_secs: i64 = row.try_get("sync_frequency_secs").unwrap_or(300);
        let last_sync_ts: Option<i64> = row.try_get("last_sync_ts").ok();
        let created_at: i64 = row.try_get("created_at").unwrap_or(0);
        let updated_at: i64 = row.try_get("updated_at").unwrap_or(0);
        let append_policy: Option<String> = row.try_get("append_policy").ok();
        let sent_folder_hint: Option<String> = row.try_get("sent_folder_hint").ok();
        let color: Option<String> = row.try_get("color").ok();
        let carddav_url: Option<String> = row.try_get("carddav_url").ok();
        let caldav_url: Option<String> = row.try_get("caldav_url").ok();

        accounts.push(Account {
            id,
            email,
            provider: EmailProvider::from_str(&provider_s),
            display_name,
            imap_host,
            imap_port: imap_port_i as u16,
            smtp_host,
            smtp_port: smtp_port_i as u16,
            credentials_encrypted,
            enabled: enabled_i != 0,
            sync_frequency_secs,
            last_sync_ts,
            created_at,
            updated_at,
            append_policy,
            sent_folder_hint,
            color,
            carddav_url,
            caldav_url,
            password: String::new(),
        });
    }

    Ok(accounts)
}

/// Get account by ID
pub async fn get_account(pool: &SqlitePool, account_id: &str) -> Result<Option<Account>> {
    let row_opt = sqlx::query("SELECT * FROM accounts WHERE id = ?")
        .bind(account_id)
        .fetch_optional(pool)
        .await?;

    Ok(match row_opt {
        Some(row) => {
            let id: String = row.try_get("id").unwrap_or_default();
            let email: String = row.try_get("email").unwrap_or_default();
            let provider_s: String = row.try_get("provider").unwrap_or_else(|_| "custom".to_string());
            let display_name: Option<String> = row.try_get("display_name").ok();
            let imap_host: String = row.try_get("imap_host").unwrap_or_default();
            let imap_port_i: i64 = row.try_get("imap_port").unwrap_or(993);
            let smtp_host: String = row.try_get("smtp_host").unwrap_or_default();
            let smtp_port_i: i64 = row.try_get("smtp_port").unwrap_or(587);
            let credentials_encrypted: String = row.try_get("credentials_encrypted").unwrap_or_default();
            let enabled_i: i64 = row.try_get("enabled").unwrap_or(1);
            let sync_frequency_secs: i64 = row.try_get("sync_frequency_secs").unwrap_or(300);
            let last_sync_ts: Option<i64> = row.try_get("last_sync_ts").ok();
            let created_at: i64 = row.try_get("created_at").unwrap_or(0);
            let updated_at: i64 = row.try_get("updated_at").unwrap_or(0);
            let append_policy: Option<String> = row.try_get("append_policy").ok();
            let sent_folder_hint: Option<String> = row.try_get("sent_folder_hint").ok();
            let color: Option<String> = row.try_get("color").ok();
            let carddav_url: Option<String> = row.try_get("carddav_url").ok();
            let caldav_url: Option<String> = row.try_get("caldav_url").ok();

            let mut acc = Account {
                id,
                email,
                provider: EmailProvider::from_str(&provider_s),
                display_name,
                imap_host,
                imap_port: imap_port_i as u16,
                smtp_host,
                smtp_port: smtp_port_i as u16,
                credentials_encrypted,
                enabled: enabled_i != 0,
                sync_frequency_secs,
                last_sync_ts,
                created_at,
                updated_at,
                append_policy,
                sent_folder_hint,
                color,
                carddav_url,
                caldav_url,
                password: String::new(),
            };

            if !acc.credentials_encrypted.is_empty() {
                acc = acc.with_password()?;
            }

            Some(acc)
        }
        None => None,
    })
}

/// Delete account
pub async fn delete_account(pool: &SqlitePool, account_id: &str) -> Result<bool> {
    let result = sqlx::query!("DELETE FROM accounts WHERE id = ?", account_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Update last sync timestamp
pub async fn update_last_sync(pool: &SqlitePool, account_id: &str) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    sqlx::query!(
        "UPDATE accounts SET last_sync_ts = ?, updated_at = ? WHERE id = ?",
        now,
        now,
        account_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update an existing account (partial). Returns updated Account.
pub async fn update_account(
    pool: &SqlitePool,
    account_id: &str,
    email: Option<String>,
    password: Option<String>,
    provider: Option<EmailProvider>,
    display_name: Option<Option<String>>,
    imap_host: Option<String>,
    imap_port: Option<u16>,
    smtp_host: Option<String>,
    smtp_port: Option<u16>,
    enabled: Option<bool>,
    append_policy: Option<Option<String>>,
    sent_folder_hint: Option<Option<String>>,
    color: Option<String>,
) -> Result<Option<Account>> {
    // Fetch current
    let current_opt = get_account(pool, account_id).await?;
    let mut current = match current_opt { Some(c) => c, None => return Ok(None) };
    // Apply changes in memory
    if let Some(e) = email { current.email = e; }
    if let Some(pv) = provider { current.provider = pv; }
    if let Some(dn_opt) = display_name { current.display_name = dn_opt; }
    if let Some(ih) = imap_host { current.imap_host = ih; }
    if let Some(ip) = imap_port { current.imap_port = ip; }
    if let Some(sh) = smtp_host { current.smtp_host = sh; }
    if let Some(sp) = smtp_port { current.smtp_port = sp; }
    if let Some(en) = enabled { current.enabled = en; }
    if let Some(ap_opt) = append_policy { current.append_policy = ap_opt; }
    if let Some(sf_opt) = sent_folder_hint { current.sent_folder_hint = sf_opt; }
    if let Some(c) = color { current.color = Some(c); }
    if let Some(pass) = password { current.credentials_encrypted = Account::encode_credentials(&current.email, &pass); current.password = pass; }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;
    // Persist (use dynamic query to avoid compile-time DB schema checks)
    sqlx::query(
        r#"UPDATE accounts SET email = ?, provider = ?, display_name = ?, imap_host = ?, imap_port = ?, smtp_host = ?, smtp_port = ?, credentials_encrypted = ?, enabled = ?, append_policy = ?, sent_folder_hint = ?, color = COALESCE(?, color), updated_at = ? WHERE id = ?"#
    )
    .bind(&current.email)
    .bind(current.provider.as_str())
    .bind(current.display_name.as_deref())
    .bind(&current.imap_host)
    .bind(current.imap_port as i64)
    .bind(&current.smtp_host)
    .bind(current.smtp_port as i64)
    .bind(&current.credentials_encrypted)
    .bind(if current.enabled {1} else {0})
    .bind(current.append_policy.as_deref())
    .bind(current.sent_folder_hint.as_deref())
    .bind(current.color.as_deref())
    .bind(now)
    .bind(account_id)
    .execute(pool)
    .await?;
    Ok(Some(current))
}
