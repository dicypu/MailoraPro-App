use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::SqlitePool;

#[derive(Deserialize)]
pub struct SnoozePayload {
    pub until: String, // ISO8601
}

pub async fn snooze_message(
    State(pool): State<SqlitePool>,
    Path((account_id, folder, uid)): Path<(String, String, i64)>,
    Json(payload): Json<SnoozePayload>,
) -> impl IntoResponse {
    // Validate datetime? SQLite allows text, so verify it's valid ISO partial.
    // Ideally we parse it to ensure safety.
    if chrono::DateTime::parse_from_rfc3339(&payload.until).is_err() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "error": "Invalid date format"}))).into_response();
    }

    let sql = "UPDATE messages SET snoozed_until = ? WHERE account_id = ? AND folder = ? AND uid = ?";
    match sqlx::query(sql)
        .bind(&payload.until)
        .bind(account_id)
        .bind(folder)
        .bind(uid)
        .execute(&pool)
        .await
    {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": e.to_string()}))).into_response(),
    }
}

pub async fn unsnooze_message(
    State(pool): State<SqlitePool>,
    Path((account_id, folder, uid)): Path<(String, String, i64)>,
) -> impl IntoResponse {
    let sql = "UPDATE messages SET snoozed_until = NULL WHERE account_id = ? AND folder = ? AND uid = ?";
    match sqlx::query(sql)
        .bind(account_id)
        .bind(folder)
        .bind(uid)
        .execute(&pool)
        .await
    {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": e.to_string()}))).into_response(),
    }
}
