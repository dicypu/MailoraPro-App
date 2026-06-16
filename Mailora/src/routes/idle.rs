/// IDLE Watcher Management Endpoints
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::stream::Stream;
use serde::Serialize;
use sqlx::SqlitePool;
use std::convert::Infallible;
use std::sync::Arc;

use crate::services::{account_service, idle_watcher_service::IdleWatcherManager};

/// POST /idle/start/:account_id - Start IDLE watcher for account
pub async fn start_idle_watcher(
    State(pool): State<SqlitePool>,
    State(idle_manager): State<Arc<IdleWatcherManager>>,
    Path(account_id): Path<String>,
) -> Result<Json<IdleResponse>, (StatusCode, String)> {
    let account = account_service::get_account(&pool, &account_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Account {} not found", account_id),
            )
        })?;

    tracing::info!("Starting IDLE watcher for account: {}", account.email);

    idle_manager
        .start_watcher(account.clone())
        .await
        .map_err(|e| {
            tracing::error!("Failed to start IDLE watcher: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to start watcher: {}", e),
            )
        })?;

    Ok(Json(IdleResponse {
        success: true,
        message: format!("IDLE watcher started for {}", account.email),
        account_id: account.id,
    }))
}

/// POST /idle/stop/:account_id - Stop IDLE watcher
pub async fn stop_idle_watcher(
    State(idle_manager): State<Arc<IdleWatcherManager>>,
    Path(account_id): Path<String>,
) -> Result<Json<IdleResponse>, (StatusCode, String)> {
    tracing::info!("Stopping IDLE watcher for account: {}", account_id);

    idle_manager.stop_watcher(&account_id).await.map_err(|e| {
        tracing::error!("Failed to stop IDLE watcher: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to stop watcher: {}", e),
        )
    })?;

    Ok(Json(IdleResponse {
        success: true,
        message: format!("IDLE watcher stopped for {}", account_id),
        account_id,
    }))
}

/// GET /idle/status - Get status of all watchers
pub async fn idle_status(
    State(idle_manager): State<Arc<IdleWatcherManager>>,
) -> Result<Json<IdleStatusResponse>, (StatusCode, String)> {
    let active_count = idle_manager.active_count().await;

    Ok(Json(IdleStatusResponse {
        active_watchers: active_count,
    }))
}

/// GET /idle/events - SSE stream of IDLE events
pub async fn idle_events_stream(
    State(idle_manager): State<Arc<IdleWatcherManager>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = idle_manager.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    yield Ok(Event::default().data(json));
                }
                Err(e) => {
                    tracing::error!("Event stream error: {}", e);
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Debug, Serialize)]
pub struct IdleResponse {
    pub success: bool,
    pub message: String,
    pub account_id: String,
}

#[derive(Debug, Serialize)]
pub struct IdleStatusResponse {
    pub active_watchers: usize,
}
