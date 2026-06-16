use crate::models::calendar::{CalendarEvent};
use crate::pim::dav_client::{DavClient, DavResource};
use crate::pim::ical::{parse_ical, ParsedEvent};
use anyhow::{anyhow, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashMap;
use tracing::{error, info, warn};

pub async fn sync_caldav(pool: &SqlitePool, account_id: &str) -> Result<()> {
    // 1. Fetch account credentials and URL
    // Here we'll just mock the fetching similar to carddav_service
    let mut account = get_account_creds(pool, account_id).await?;
    if account.caldav_url.is_none() || account.caldav_url.as_ref().unwrap().is_empty() {
        if let Some(discovered_url) = discover_caldav_url(&account).await {
            if let Err(e) = save_caldav_url(pool, account_id, &discovered_url).await {
                warn!("Failed to save discovered CalDAV URL for account {}: {}", account_id, e);
            }
            account.caldav_url = Some(discovered_url);
        } else {
            return Ok(()); // Nothing to sync, discovery failed
        }
    }
    
    let base_url = account.caldav_url.unwrap();
    let client = DavClient::new(&base_url, &account.email, &account.password);
    
    // 2. Discover principal & calendar-home-set
    let principal_url = match client.discover_principal().await {
        Ok(url) => url,
        Err(e) => {
            error!("CalDAV discover failed for {}: {}", account_id, e);
            update_sync_state(pool, account_id, Some(&e.to_string())).await?;
            return Err(e);
        }
    };
    
    // 3. Discover Calendars
    let calendars = match client.discover_calendars(&principal_url).await {
         Ok(c) => c,
         Err(e) => {
              error!("Failed to discover calendars for {}: {}", account_id, e);
              update_sync_state(pool, account_id, Some(&e.to_string())).await?;
              return Err(e);
         }
    };
    
    for cal in calendars {
        // Skip non-calendar collections for now (e.g. outbox/inbox for scheduling)
        // We will just try to sync any valid collection that returns VEVENT components via report
        if cal.href.contains("/outbox") || cal.href.contains("/inbox") {
            continue;
        }

        // Upsert calendar into DB
        let cal_id = upsert_calendar_db(pool, account_id, &cal).await?;

        // 4. Report all events inside calendar
        let remote_events = match client.report_calendar(&cal.href).await {
             Ok(r) => r,
             Err(e) => {
                 warn!("Could not report calendar {}: {}", cal.href, e);
                 continue;
             }
        };
        
        // Convert to map
        let mut remote_map: HashMap<String, String> = HashMap::new();
        for r in remote_events {
             if let Some(etag) = r.etag {
                  remote_map.insert(r.href, etag);
             }
        }
        
        // 5. Fetch local events for this calendar
        let local_events = get_local_events(pool, &cal_id).await?;
        let mut local_map: HashMap<String, String> = HashMap::new();
        for le in &local_events {
            // we only map synced/conflict/pending_update items. pending_create has no href usually
            if !le.href.is_empty() {
                local_map.insert(le.href.clone(), le.etag.clone().unwrap_or_default());
            }
        }

        // 6. Push local pending creates to server
        for le in local_events.iter().filter(|c| c.sync_status == "pending_create") {
             let new_href = format!("{}/{}.ics", cal.href.trim_end_matches('/'), le.uid);
             match client.put(&new_href, &le.raw_ical, "text/calendar", None).await {
                  Ok(Some(new_etag)) => {
                       update_event_etag_href(pool, &le.id, &new_etag, &new_href).await?;
                  }
                  Ok(None) => {
                       // Refresh to get etag
                       if let Ok((_, Some(et))) = client.get(&new_href).await {
                             update_event_etag_href(pool, &le.id, &et, &new_href).await?;
                       }
                  }
                  Err(e) => error!("Failed to create event {}: {}", le.uid, e)
             }
        }

        // 7. DIFF & SYNC
        for (remote_href, remote_etag) in &remote_map {
            let local_event = local_events.iter().find(|c| c.href == *remote_href);
            
            if let Some(le) = local_event {
                 if le.sync_status == "pending_update" {
                      // Conflict or Update
                      if le.etag.as_deref() == Some(remote_etag) {
                           // Safe to push
                           match client.put(remote_href, &le.raw_ical, "text/calendar", le.etag.as_deref()).await {
                                Ok(Some(new_etag)) => { update_event_etag_href(pool, &le.id, &new_etag, remote_href).await?; },
                                Ok(None) => {
                                     if let Ok((_, Some(et))) = client.get(remote_href).await {
                                           update_event_etag_href(pool, &le.id, &et, remote_href).await?;
                                     }
                                }
                                Err(e) => error!("Failed to update event {}: {}", remote_href, e)
                           }
                      } else {
                           // Conflict detection!
                           warn!("CalDAV conflict detected for {}", remote_href);
                           if let Ok((_remote_ics, _)) = client.get(remote_href).await {
                               // Just reset to sync status conflict for now
                               sqlx::query("UPDATE calendar_events SET sync_status = 'conflict', updated_at = ? WHERE id = ?")
                                 .bind(Utc::now().to_rfc3339())
                                 .bind(&le.id)
                                 .execute(pool).await?;
                           }
                      }
                 } else if le.sync_status == "pending_delete" {
                      // Push delete
                      let _ = client.delete(remote_href, le.etag.as_deref()).await;
                      sqlx::query("DELETE FROM calendar_events WHERE id = ?").bind(&le.id).execute(pool).await?;
                 } else if le.etag.as_deref() != Some(remote_etag) {
                      // Fetch updated
                      if let Ok((ics, etag)) = client.get(remote_href).await {
                           upsert_event_db(pool, &cal_id, remote_href, etag.as_deref(), &ics).await?;
                      }
                 }
            } else {
                 // New remote event
                 if let Ok((ics, etag)) = client.get(remote_href).await {
                      upsert_event_db(pool, &cal_id, remote_href, etag.as_deref(), &ics).await?;
                 }
            }
        }
        
        // 8. Find local items not on server anymore
        for le in &local_events {
            if !le.href.is_empty() && !remote_map.contains_key(&le.href) {
                if le.sync_status == "synced" {
                     sqlx::query("DELETE FROM calendar_events WHERE id = ?").bind(&le.id).execute(pool).await?;
                }
            }
        }
    }
    
    update_sync_state(pool, account_id, None).await?;
    info!("Successfully synced CalDAV for account {}", account_id);
    Ok(())
}

async fn get_account_creds(pool: &SqlitePool, account_id: &str) -> Result<AccountCreds> {
    #[derive(sqlx::FromRow)]
    struct RawCreds {
        email: String,
        credentials_encrypted: String,
        caldav_url: Option<String>,
    }
    
    let row: RawCreds = sqlx::query_as(
        "SELECT email, credentials_encrypted, caldav_url FROM accounts WHERE id = ?"
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("Account not found"))?;
    
    let (_, password) = crate::models::account::Account::decode_credentials(&row.credentials_encrypted)
        .map_err(|e| anyhow!("Failed to decode credentials: {}", e))?;
    
    Ok(AccountCreds {
        email: row.email,
        password,
        caldav_url: row.caldav_url,
    })
}

struct AccountCreds {
    email: String,
    password: String,
    caldav_url: Option<String>,
}

async fn update_sync_state(pool: &SqlitePool, account_id: &str, err: Option<&str>) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"INSERT INTO caldav_sync_state (account_id, last_synced_at, error_msg)
           VALUES (?, ?, ?)
           ON CONFLICT(account_id) DO UPDATE SET last_synced_at = excluded.last_synced_at, error_msg = excluded.error_msg"#
    )
    .bind(account_id)
    .bind(&now)
    .bind(err)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_calendar_db(pool: &SqlitePool, account_id: &str, cal: &DavResource) -> Result<String> {
    let now = Utc::now().to_rfc3339();
    
    #[derive(sqlx::FromRow)]
    struct CalIdRow { id: String }
    
    let existing: Option<CalIdRow> = sqlx::query_as("SELECT id FROM calendars WHERE account_id = ? AND url = ?")
        .bind(account_id)
        .bind(&cal.href)
        .fetch_optional(pool)
        .await?;
        
    if let Some(row) = existing {
         sqlx::query("UPDATE calendars SET ctag = ?, updated_at = ? WHERE id = ?")
             .bind(cal.etag.clone())
             .bind(&now)
             .bind(&row.id)
             .execute(pool).await?;
         Ok(row.id)
    } else {
         let id = uuid::Uuid::new_v4().to_string();
         // Basic display name derivation
         let name = cal.href.trim_end_matches('/').split('/').last().unwrap_or("Calendar").to_string();
         
         sqlx::query(
             r#"INSERT INTO calendars (id, account_id, url, display_name, ctag, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)"#
         )
         .bind(&id)
         .bind(account_id)
         .bind(&cal.href)
         .bind(&name)
         .bind(cal.etag.clone())
         .bind(&now)
         .bind(&now)
         .execute(pool).await?;
         
         Ok(id)
    }
}

async fn get_local_events(pool: &SqlitePool, cal_id: &str) -> Result<Vec<CalendarEvent>> {
    #[derive(sqlx::FromRow)]
    struct EventRowRaw {
        id: String,
        calendar_id: String,
        uid: String,
        href: String,
        etag: Option<String>,
        raw_ical: String,
        sync_status: String,
    }
    
    let rows: Vec<EventRowRaw> = sqlx::query_as(
        "SELECT id, calendar_id, uid, href, etag, raw_ical, sync_status FROM calendar_events WHERE calendar_id = ?"
    )
    .bind(cal_id)
    .fetch_all(pool).await?;
    
    let mut res = Vec::new();
    for r in rows {
         res.push(CalendarEvent {
              id: r.id,
              calendar_id: r.calendar_id,
              uid: r.uid,
              href: r.href,
              etag: r.etag,
              raw_ical: r.raw_ical,
              summary: None,
              description: None,
              location: None,
              dtstart: None,
              dtend: None,
              is_all_day: 0,
              timezone: None,
              rrule: None,
              status: None,
              sync_status: r.sync_status,
              created_at: String::new(),
              updated_at: String::new(),
         });
    }
    Ok(res)
}

async fn update_event_etag_href(pool: &SqlitePool, id: &str, etag: &str, href: &str) -> Result<()> {
    sqlx::query("UPDATE calendar_events SET etag = ?, href = ?, sync_status = 'synced', updated_at = ? WHERE id = ?")
        .bind(etag)
        .bind(href)
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(pool).await?;
    Ok(())
}

async fn upsert_event_db(pool: &SqlitePool, cal_id: &str, href: &str, etag: Option<&str>, ics: &str) -> Result<()> {
    let parsed_opt = parse_ical(ics);
    if parsed_opt.is_none() || parsed_opt.as_ref().unwrap().events.is_empty() {
         warn!("Could not parse iCal data from {}", href);
         return Ok(());
    }
    let parsed = parsed_opt.unwrap();
    let ev = &parsed.events[0];
    
    let default_uid = format!("mailora-{}", uuid::Uuid::new_v4());
    let uid = ev.uid.as_deref().unwrap_or(&default_uid);
    let now = Utc::now().to_rfc3339();
    
    // Check if exists
    #[derive(sqlx::FromRow)]
    struct IdRow { id: String }
    let existing: Option<IdRow> = sqlx::query_as("SELECT id FROM calendar_events WHERE calendar_id = ? AND href = ?")
         .bind(cal_id)
         .bind(href)
         .fetch_optional(pool)
         .await?;
         
    if let Some(row) = existing {
         // Update
         let mut tx = pool.begin().await?;
         
         sqlx::query(
             r#"UPDATE calendar_events 
                SET etag = ?, raw_ical = ?, summary = ?, description = ?, location = ?, 
                    dtstart = ?, dtend = ?, is_all_day = ?, timezone = ?, rrule = ?, status = ?,
                    sync_status = 'synced', updated_at = ?
                WHERE id = ?"#
         )
         .bind(etag)
         .bind(ics)
         .bind(ev.summary.as_ref())
         .bind(ev.description.as_ref())
         .bind(ev.location.as_ref())
         .bind(ev.dtstart.as_ref())
         .bind(ev.dtend.as_ref())
         .bind(if ev.is_all_day { 1 } else { 0 })
         .bind(ev.tz_id.as_ref())
         .bind(ev.rrule.as_ref())
         .bind(ev.status.as_ref())
         .bind(&now)
         .bind(&row.id)
         .execute(&mut *tx).await?;
         
         // Clear attendees/alarms and recreate
         sqlx::query("DELETE FROM event_attendees WHERE event_id = ?").bind(&row.id).execute(&mut *tx).await?;
         sqlx::query("DELETE FROM event_alarms WHERE event_id = ?").bind(&row.id).execute(&mut *tx).await?;
         
         insert_attendees_alarms(&mut tx, &row.id, ev).await?;
         tx.commit().await?;
         
    } else {
         // Insert
         let id = uuid::Uuid::new_v4().to_string();
         let mut tx = pool.begin().await?;
         
         sqlx::query(
             r#"INSERT INTO calendar_events (
                id, calendar_id, uid, href, etag, raw_ical, summary, description, location,
                dtstart, dtend, is_all_day, timezone, rrule, status, sync_status, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'synced', ?, ?)"#
         )
         .bind(&id)
         .bind(cal_id)
         .bind(uid)
         .bind(href)
         .bind(etag)
         .bind(ics)
         .bind(ev.summary.as_ref())
         .bind(ev.description.as_ref())
         .bind(ev.location.as_ref())
         .bind(ev.dtstart.as_ref())
         .bind(ev.dtend.as_ref())
         .bind(if ev.is_all_day { 1 } else { 0 })
         .bind(ev.tz_id.as_ref())
         .bind(ev.rrule.as_ref())
         .bind(ev.status.as_ref())
         .bind(&now)
         .bind(&now)
         .execute(&mut *tx).await?;
         
         insert_attendees_alarms(&mut tx, &id, ev).await?;
         tx.commit().await?;
    }
    
    Ok(())
}

async fn insert_attendees_alarms(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, event_id: &str, ev: &ParsedEvent) -> Result<()> {
    for att in &ev.attendees {
         let att_id = uuid::Uuid::new_v4().to_string();
         sqlx::query(
             "INSERT INTO event_attendees (id, event_id, email, cn, partstat, is_organizer) VALUES (?, ?, ?, ?, ?, 0)"
         )
         .bind(&att_id)
         .bind(event_id)
         .bind(&att.email)
         .bind(att.cn.as_ref())
         .bind(&att.partstat)
         .execute(&mut **tx).await?;
    }
    
    for al in &ev.alarms {
         let al_id = uuid::Uuid::new_v4().to_string();
         sqlx::query(
             "INSERT INTO event_alarms (id, event_id, action, trigger_text, description) VALUES (?, ?, ?, ?, ?)"
         )
         .bind(&al_id)
         .bind(event_id)
         .bind(&al.action)
         .bind(&al.trigger)
         .bind(al.description.as_ref())
         .execute(&mut **tx).await?;
    }
    Ok(())
}

async fn save_caldav_url(pool: &SqlitePool, account_id: &str, url: &str) -> Result<()> {
    sqlx::query("UPDATE accounts SET caldav_url = ? WHERE id = ?")
        .bind(url).bind(account_id).execute(pool).await?;
    Ok(())
}

pub async fn discover_caldav_url(account: &AccountCreds) -> Option<String> {
    let domain = account.email.split('@').nth(1)?;

    // 1. Hardcoded known providers
    let known = known_caldav_url(domain, &account.email);
    if known.is_some() { return known; }

    // 2. .well-known/caldav
    let well_known = format!("https://{}/.well-known/caldav", domain);
    if probe_url(&well_known).await { return Some(well_known); }

    // 3. HTTP fallback
    let well_known_http = format!("http://{}/.well-known/caldav", domain);
    if probe_url(&well_known_http).await { return Some(well_known_http); }

    None
}

fn known_caldav_url(domain: &str, _email: &str) -> Option<String> {
    match domain {
        "gmail.com" | "googlemail.com" => Some("https://apidata.googleusercontent.com/caldav/v2/".into()),
        "icloud.com" | "me.com" | "mac.com" => Some("https://caldav.icloud.com/".into()),
        "yahoo.com" => Some("https://caldav.calendar.yahoo.com/".into()),
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
