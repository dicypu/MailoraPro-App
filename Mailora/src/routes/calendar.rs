use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post, put, delete},
    Json, Router,
};
use tracing::error;
use serde_json::json;
use crate::models::calendar::{Calendar, CalendarEvent, EventRequest};
use chrono::Utc;

pub fn router() -> Router<sqlx::SqlitePool> {
    Router::new()
        .route("/accounts/:account_id/calendars", get(list_calendars))
        .route("/accounts/:account_id/calendars/:calendar_id/events", get(list_events))
        .route("/accounts/:account_id/calendars/:calendar_id/events", post(create_event))
        .route("/accounts/:account_id/calendars/:calendar_id/events/:event_id", put(update_event))
        .route("/accounts/:account_id/calendars/:calendar_id/events/:event_id", delete(delete_event))
        .route("/accounts/:account_id/caldav/sync", post(trigger_sync))
}

// GET /accounts/:id/calendars
// GET /accounts/:id/calendars
async fn list_calendars(
    State(pool): State<sqlx::SqlitePool>,
    Path(account_id): Path<String>,
) -> impl IntoResponse {
    let rows: Result<Vec<Calendar>, _> = sqlx::query_as("SELECT * FROM calendars WHERE account_id = ?")
        .bind(&account_id)
        .fetch_all(&pool)
        .await;
        
    let mut cals = match rows {
        Ok(c) => c,
        Err(e) => {
            error!("Fail list calendars: {}", e);
            return Json(json!({ "success": false, "error": e.to_string() })).into_response();
        }
    };

    if cals.is_empty() {
        let default_id = format!("local_{}", account_id);
        let now = Utc::now().to_rfc3339();
        
        let insert_res = sqlx::query(
            "INSERT INTO calendars (id, account_id, url, display_name, color, description, created_at, updated_at) VALUES (?, ?, 'local', 'Kişisel Takvim', '#3b82f6', 'Varsayılan Yerel Takvim', ?, ?)"
        )
        .bind(&default_id)
        .bind(&account_id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await;

        if insert_res.is_ok() {
            cals.push(Calendar {
                id: default_id,
                account_id: account_id,
                url: "local".to_string(),
                display_name: Some("Kişisel Takvim".to_string()),
                color: Some("#3b82f6".to_string()),
                description: Some("Varsayılan Yerel Takvim".to_string()),
                ctag: None,
                sync_token: None,
                created_at: now.clone(),
                updated_at: now,
            });
        }
    }

    Json(json!({ "success": true, "data": cals })).into_response()
}

// GET /accounts/:id/calendars/:cal_id/events
// GET /accounts/:id/calendars/:cal_id/events
async fn list_events(
    State(pool): State<sqlx::SqlitePool>,
    Path((_account_id, calendar_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let rows: Result<Vec<CalendarEvent>, _> = sqlx::query_as(
        "SELECT * FROM calendar_events WHERE calendar_id = ? AND sync_status != 'pending_delete'"
    )
    .bind(&calendar_id)
    .fetch_all(&pool)
    .await;
    
    match rows {
        Ok(evs) => Json(json!({ "success": true, "data": evs })).into_response(),
        Err(e) => {
            error!("Fail list events: {}", e);
            Json(json!({ "success": false, "error": e.to_string() })).into_response()
        }
    }
}

// POST /accounts/:id/calendars/:cal_id/events
// POST /accounts/:id/calendars/:cal_id/events
async fn create_event(
    State(pool): State<sqlx::SqlitePool>,
    Path((_account_id, calendar_id)): Path<(String, String)>,
    Json(req): Json<EventRequest>,
) -> impl IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    let uid = format!("mailora-{}", uuid::Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    
    // In Sprint 1 we just serialize minimum fields and set pending_create. 
    // CalDAV sync background worker will push to server.
    
    let is_all_day_i64 = if req.is_all_day { 1 } else { 0 };
    // Basic serializer for raw_ical
    let raw_ical = crate::pim::ical::serialize_ical(
        &uid,
        &req.summary,
        req.description.as_deref(),
        req.location.as_deref(),
        &req.dtstart,
        &req.dtend,
        req.is_all_day,
        req.timezone.as_deref(),
        req.rrule.as_deref()
    );
    
    let res = sqlx::query(
        r#"INSERT INTO calendar_events (
            id, calendar_id, uid, href, raw_ical, summary, description, location, 
            dtstart, dtend, is_all_day, timezone, rrule, status, sync_status, created_at, updated_at
        ) VALUES (?, ?, ?, '', ?, ?, ?, ?, ?, ?, ?, ?, ?, 'CONFIRMED', 'pending_create', ?, ?)"#
    )
    .bind(&id)
    .bind(&calendar_id)
    .bind(&uid)
    .bind(&raw_ical)
    .bind(&req.summary)
    .bind(req.description)
    .bind(req.location)
    .bind(&req.dtstart)
    .bind(&req.dtend)
    .bind(is_all_day_i64)
    .bind(req.timezone)
    .bind(req.rrule)
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await;
    
    match res {
        Ok(_) => Json(json!({ "success": true, "id": id })).into_response(),
        Err(e) => {
             error!("Create event fail: {}", e);
             Json(json!({ "success": false, "error": e.to_string() })).into_response()
        }
    }
}

// PUT /accounts/:id/calendars/:cal_id/events/:event_id
// PUT /accounts/:id/calendars/:cal_id/events/:event_id
async fn update_event(
    State(pool): State<sqlx::SqlitePool>,
    Path((_account_id, _calendar_id, event_id)): Path<(String, String, String)>,
    Json(req): Json<EventRequest>, // In real app, we need uid
) -> impl IntoResponse {
    let now = Utc::now().to_rfc3339();
    let is_all_day_i64 = if req.is_all_day { 1 } else { 0 };
    
    // For raw_ical, we should fetch old UID. Mocking a generate here.
    let uid = uuid::Uuid::new_v4().to_string(); 
    let raw_ical = crate::pim::ical::serialize_ical(
        &uid,
        &req.summary,
        req.description.as_deref(),
        req.location.as_deref(),
        &req.dtstart,
        &req.dtend,
        req.is_all_day,
        req.timezone.as_deref(),
        req.rrule.as_deref()
    );

    let res = sqlx::query(
        r#"UPDATE calendar_events SET
            summary = ?, description = ?, location = ?, dtstart = ?, dtend = ?, is_all_day = ?,
            timezone = ?, rrule = ?, raw_ical = ?, sync_status = 'pending_update', updated_at = ?
           WHERE id = ?"#
    )
    .bind(&req.summary)
    .bind(req.description)
    .bind(req.location)
    .bind(&req.dtstart)
    .bind(&req.dtend)
    .bind(is_all_day_i64)
    .bind(req.timezone)
    .bind(req.rrule)
    .bind(&raw_ical)
    .bind(&now)
    .bind(&event_id)
    .execute(&pool)
    .await;
    
    match res {
        Ok(_) => Json(json!({ "success": true })).into_response(),
        Err(e) => {
            error!("Update event fail: {}", e);
            Json(json!({ "success": false, "error": e.to_string() })).into_response()
        }
    }
}

// DELETE /accounts/:id/calendars/:cal_id/events/:event_id
// DELETE /accounts/:id/calendars/:cal_id/events/:event_id
async fn delete_event(
    State(pool): State<sqlx::SqlitePool>,
    Path((_account_id, _calendar_id, event_id)): Path<(String, String, String)>,
) -> impl IntoResponse {
    let now = Utc::now().to_rfc3339();
    
    // Mark as pending_delete so CalDAV sync deletes it on server
    let res = sqlx::query("UPDATE calendar_events SET sync_status = 'pending_delete', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&event_id)
        .execute(&pool)
        .await;
        
    match res {
         Ok(_) => Json(json!({ "success": true })).into_response(),
         Err(e) => {
             error!("Delete event fail: {}", e);
             Json(json!({ "success": false, "error": e.to_string() })).into_response()
         }
    }
}

// POST /accounts/:account_id/caldav/sync
// POST /accounts/:account_id/caldav/sync
async fn trigger_sync(
    State(pool): State<sqlx::SqlitePool>,
    Path(account_id): Path<String>,
) -> impl IntoResponse {
    // Start background sync
    tokio::spawn(async move {
         let _ = crate::services::caldav_service::sync_caldav(&pool, &account_id).await;
    });
    
    Json(json!({ "success": true, "message": "Sync started" })).into_response()
}
