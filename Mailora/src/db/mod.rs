// filepath: /mailora-hub-imap/mailora-hub-imap/src/db/mod.rs
use anyhow::{Result, anyhow};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::{Pool, Sqlite, SqlitePool};
use std::fs;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
pub struct Database {
    pub pool: Pool<Sqlite>,
}

#[allow(dead_code)]
impl Database {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .create_if_missing(true);
            
        let pool = SqlitePool::connect_with(options).await?;
        
        // Log WAL status
        let row: (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await?;
        tracing::info!("SQLite Journal Mode: {}", row.0);

        Ok(Self { pool })
    }
}

pub async fn check_and_fix_schema(pool: &SqlitePool) -> Result<()> {
    // Check if `accounts` exists and has `provider` column
    // logic: try to select provider from accounts limit 1
    // if error contains "no such column", then we have the legacy schema.
    let check = sqlx::query("SELECT provider FROM accounts LIMIT 1").fetch_optional(pool).await;
    match check {
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("no such column") {
                tracing::warn!("Detected legacy schema (missing provider column). Backing up and cleaning for migration.");
                let ts = now_epoch();
                let backup_name = format!("accounts_backup_legacy_{}", ts);
                let sql = format!("ALTER TABLE accounts RENAME TO {}", backup_name);
                sqlx::query(&sql).execute(pool).await?;
                tracing::info!("Renamed broken accounts table to {}", backup_name);
            } else if msg.contains("no such table") {
                // Table doesn't exist, fresh install, ignore
            } else {
                // Other error, let it propagate? Or ignore?
                tracing::warn!("Schema check warning: {}", msg);
            }
        }
        Ok(_) => {
            // Success means column exists, or table empty but column exists
        }
    }
    Ok(())
}

fn tokenize_sql_statements(sql: &str) -> Vec<String> {
    // Tokenize SQL into statements, preserving CREATE TRIGGER ... END; blocks
    let mut stmts: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut in_trigger = false;
    let mut i = 0;
    let bytes = sql.as_bytes();
    while i < bytes.len() {
        let c = bytes[i] as char;
        let next = if i + 1 < bytes.len() { Some(bytes[i + 1] as char) } else { None };
        // Handle comments
        if !in_single && !in_double && !in_line_comment && !in_block_comment {
            if c == '-' && next == Some('-') {
                in_line_comment = true;
            } else if c == '/' && next == Some('*') {
                in_block_comment = true;
            }
        }
        // Append char
        buf.push(c);
        // State transitions
        if in_line_comment {
            if c == '\n' { in_line_comment = false; }
            i += 1;
            continue;
        }
        if in_block_comment {
            if c == '*' && next == Some('/') { in_block_comment = false; buf.push('/'); i += 2; continue; }
            i += 1;
            continue;
        }
        if !in_double && c == '\'' { in_single = !in_single; i += 1; continue; }
        if !in_single && c == '"' { in_double = !in_double; i += 1; continue; }

        // Detect start of CREATE TRIGGER (case-insensitive) when not inside quotes
        if !in_single && !in_double && !in_trigger {
            // Look back a small window to see if we just completed "CREATE TRIGGER"
            let upper_tail = buf.to_uppercase();
            if upper_tail.ends_with("CREATE TRIGGER") || upper_tail.contains("CREATE TRIGGER ") {
                in_trigger = true;
            }
        }

        // Statement termination logic
        if !in_single && !in_double {
            if in_trigger {
                // Check for END; pattern case-insensitive
                let tail = buf.trim_end();
                let tail_upper = tail.to_uppercase();
                if tail_upper.ends_with("END;") || tail_upper.ends_with("END ;") {
                    // finalize trigger
                    let stmt = buf.trim().to_string();
                    if !stmt.is_empty() { stmts.push(stmt); }
                    buf.clear();
                    in_trigger = false;
                }
            } else if c == ';' {
                let stmt = buf.trim().to_string();
                if !stmt.is_empty() { stmts.push(stmt); }
                buf.clear();
            }
        }
        i += 1;
    }
    let tail = buf.trim();
    if !tail.is_empty() { stmts.push(tail.to_string()); }
    stmts
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir("migrations")?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.path());
    for e in entries {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("sql") {
            let raw = fs::read_to_string(&p)?;
            let stmts = tokenize_sql_statements(&raw);
            for s in stmts {
                if s.is_empty() { continue; }
                if let Err(err) = sqlx::query(&s).execute(pool).await {
                    let msg = err.to_string();
                    let benign = msg.contains("already exists")
                        || msg.contains("duplicate column name")
                        || (msg.contains("no such table: messages_fts") && s.to_uppercase().contains("DROP TABLE"))
                        || msg.contains("cannot add a PRIMARY KEY")
                        || msg.contains("duplicate")
                        || msg.contains("not unique")
                        || msg.contains("no such column")
                        || msg.contains("incomplete input")
                        || msg.contains("cannot commit - no transaction is active");
                    if !benign {
                        return Err(anyhow!("migration failed at {}: {}\nstmt: {}", p.display(), msg, s));
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn seed_account(pool: &SqlitePool) -> Result<()> {
    let email = match std::env::var("IMAP_EMAIL") { Ok(v) => v, Err(_) => return Ok(()) };
    let host = match std::env::var("IMAP_HOST") { Ok(v) => v, Err(_) => return Ok(()) };
    let port: i64 = std::env::var("IMAP_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(993);
    let smtp_host = std::env::var("SMTP_HOST").unwrap_or(host.clone());
    let smtp_port: i64 = std::env::var("SMTP_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(587);
    let now = now_epoch();
    sqlx::query(r#"INSERT OR IGNORE INTO accounts(
        id, org_id, email, imap_host, imap_port, smtp_host, smtp_port, auth_type, use_ssl, created_at, updated_at
    ) VALUES (1,1,?,?,?,?,?,'LOGIN',1,?,?)"#)
        .bind(&email)
        .bind(&host)
        .bind(port)
        .bind(&smtp_host)
        .bind(smtp_port)
        .bind(now)
        .bind(now)
        .execute(pool).await?;
    Ok(())
}

pub fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

pub async fn normalize_legacy_dates(pool: &SqlitePool) -> Result<()> {
    tracing::info!("Normalizing legacy message dates to RFC 3339 standard...");
    
    #[derive(sqlx::FromRow)]
    struct DateRow {
        id: i64,
        date: Option<String>,
    }
    
    let rows: Vec<DateRow> = sqlx::query_as("SELECT id, date FROM messages WHERE date IS NOT NULL")
        .fetch_all(pool)
        .await?;

    let mut update_count = 0;
    for row in rows {
        if let Some(ref d_str) = row.date {
            let d_str_trimmed = d_str.trim();
            if d_str_trimmed.is_empty() {
                continue;
            }
            
            // If already ISO 8601 / RFC 3339 format, skip
            if chrono::DateTime::parse_from_rfc3339(d_str_trimmed).is_ok() {
                continue;
            }

            // Parse RFC 2822 format (e.g. "Wed, 8 Mar 2023 21:12:00 +0000")
            if let Ok(parsed) = chrono::DateTime::parse_from_rfc2822(d_str_trimmed) {
                let normalized = parsed.to_rfc3339();
                sqlx::query("UPDATE messages SET date = ? WHERE id = ?")
                    .bind(&normalized)
                    .bind(row.id)
                    .execute(pool)
                    .await?;
                update_count += 1;
                continue;
            }

            // Fallback: try parsing generic dates with a few standard patterns if needed
        }
    }
    
    if update_count > 0 {
        tracing::info!("Successfully normalized {} legacy message dates.", update_count);
    } else {
        tracing::info!("All message dates are already normalized.");
    }
    Ok(())
}
