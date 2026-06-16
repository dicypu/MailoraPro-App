use std::time::Duration;
use sqlx::SqlitePool;
use tracing::{info, warn};

use crate::models::account::EmailProvider;
use crate::services::{account_service, message_sync_service};

const CARDDAV_SYNC_INTERVAL_SECS: i64 = 900; // 15 dakika

/// Starts a lightweight sync scheduler. Every tick it iterates accounts and runs a delta sync.
pub fn start(pool: SqlitePool) {
    tokio::spawn(async move {
        loop {
            let tick_start = std::time::Instant::now();
            match account_service::list_accounts(&pool).await {
                Ok(accounts) => {
                    for acc_in in accounts {
                        // Skip disabled
                        if !acc_in.enabled { continue; }
                        // Skip Gmail for now (user request)
                        if matches!(acc_in.provider, EmailProvider::Gmail) { continue; }

                        // Decode credentials; skip if empty/invalid
                        let mut acc = acc_in.clone();
                        if acc.password.is_empty() {
                            match acc.clone().with_password() {
                                Ok(a) => { acc = a; },
                                Err(e) => { warn!(email=%acc.email, error=%e.to_string(), "scheduler: invalid credentials, skipping"); continue; }
                            }
                        }
                        if acc.password.is_empty() {
                            warn!(email=%acc.email, "scheduler: empty password, skipping");
                            continue;
                        }

                        // Respect per-account frequency for e-mail sync
                        let should_sync_email = if let Some(last) = acc.last_sync_ts {
                            let now = chrono::Utc::now().timestamp();
                            now - last >= acc.sync_frequency_secs
                        } else { true };

                        if should_sync_email {
                            let p = pool.clone();
                            let acc_clone = acc.clone();
                            tokio::spawn(async move {
                                match message_sync_service::sync_account_messages(&p, &acc_clone).await {
                                    Ok(stats) => {
                                        info!(email=%acc_clone.email, folders=%stats.len(), "scheduled email sync completed");
                                        let _ = crate::services::account_service::update_last_sync(&p, &acc_clone.id).await;
                                    }
                                    Err(e) => warn!(email=%acc_clone.email, error=%e.to_string(), "scheduled email sync failed"),
                                }
                            });
                        }

                        // ── CardDAV sync (15 dakikada bir) ────────────────────────
                        let should_sync_carddav = {
                            let last_carddav: Option<String> = sqlx::query_scalar(
                                "SELECT last_synced_at FROM carddav_sync_state WHERE account_id = ? ORDER BY last_synced_at DESC LIMIT 1"
                            ).bind(&acc.id).fetch_optional(&pool).await.unwrap_or(None);

                            match last_carddav {
                                None => true,
                                Some(ts) => {
                                    let last = chrono::DateTime::parse_from_rfc3339(&ts)
                                        .map(|d| d.timestamp()).unwrap_or(0);
                                    chrono::Utc::now().timestamp() - last >= CARDDAV_SYNC_INTERVAL_SECS
                                }
                            }
                        };

                        // ── CalDAV sync (15 dakikada bir) ────────────────────────
                        let should_sync_caldav = {
                            let last_caldav: Option<String> = sqlx::query_scalar(
                                "SELECT last_synced_at FROM caldav_sync_state WHERE account_id = ? ORDER BY last_synced_at DESC LIMIT 1"
                            ).bind(&acc.id).fetch_optional(&pool).await.unwrap_or(None);

                            match last_caldav {
                                None => true,
                                Some(ts) => {
                                    let last = chrono::DateTime::parse_from_rfc3339(&ts)
                                        .map(|d| d.timestamp()).unwrap_or(0);
                                    chrono::Utc::now().timestamp() - last >= CARDDAV_SYNC_INTERVAL_SECS
                                }
                            }
                        };

                        if should_sync_carddav {
                            let p = pool.clone();
                            let acc_id = acc.id.clone();
                            let email = acc.email.clone();
                            tokio::spawn(async move {
                                match crate::services::carddav_service::sync_account(&p, &acc_id).await {
                                    Ok(stats) => info!(
                                        email=%email,
                                        synced=%stats.synced, created=%stats.created,
                                        updated=%stats.updated, deleted=%stats.deleted,
                                        conflicts=%stats.conflicts,
                                        "scheduled CardDAV sync completed"
                                    ),
                                    Err(e) => {
                                        // Don't spam — CardDAV URL may simply not be configured
                                        if !e.to_string().contains("No CardDAV") {
                                            warn!(email=%email, error=%e.to_string(), "scheduled CardDAV sync failed");
                                        }
                                    }
                                }
                            });
                        }

                        if should_sync_caldav {
                            let p = pool.clone();
                            let acc_id = acc.id.clone();
                            let email = acc.email.clone();
                            tokio::spawn(async move {
                                if let Err(e) = crate::services::caldav_service::sync_caldav(&p, &acc_id).await {
                                    if !e.to_string().contains("No CalDAV") && !e.to_string().contains("not configured") {
                                        warn!(email=%email, error=%e.to_string(), "scheduled CalDAV sync failed");
                                    }
                                } else {
                                    info!(email=%email, "scheduled CalDAV sync completed");
                                }
                            });
                        }
                    }
                }
                Err(e) => warn!("scheduler: list_accounts failed: {}", e),
            }
            // Body cache GC
            crate::services::message_body_service::gc(&pool, 5000).await;
            // sleep up to 60s total per tick
            let elapsed = tick_start.elapsed();
            let sleep_ms = 60_000u64.saturating_sub(elapsed.as_millis() as u64);
            tokio::time::sleep(Duration::from_millis(sleep_ms.max(1))).await;
        }
    });
}
