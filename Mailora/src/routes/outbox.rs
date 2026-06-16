use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::SqlitePool;
use serde_json::json;
use crate::rbac::AuthUser;
use crate::models::outbox::OutboxEmail;

/// GET /outbox
/// Returns all outbox emails, optionally filtered by account_id.
/// If user is not Admin, restricts to user's assigned accounts.
pub async fn list_outbox(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let account_id_opt = params.get("account_id").cloned();

    // Query builder/logic
    let mut sql = "SELECT id, account_id, to_addr, subject, body, status, retries, last_error, 
                  CAST(strftime('%s', created_at) AS INTEGER) as created_at, 
                  CAST(strftime('%s', updated_at) AS INTEGER) as updated_at
                  FROM outbox WHERE 1=1".to_string();
    
    // Security restriction for non-Admin
    if auth_user.role != "Admin" {
        sql.push_str(" AND account_id IN (SELECT account_id FROM user_accounts WHERE user_id = ?)");
    }

    if account_id_opt.is_some() {
        sql.push_str(" AND account_id = ?");
    }

    sql.push_str(" ORDER BY created_at DESC");

    let mut q = sqlx::query_as::<_, OutboxEmail>(&sql);

    // Bind parameters in correct order
    if auth_user.role != "Admin" {
        q = q.bind(auth_user.id);
    }
    if let Some(ref acc_id) = account_id_opt {
        q = q.bind(acc_id);
    }

    match q.fetch_all(&pool).await {
        Ok(emails) => Json(json!({ "ok": true, "messages": emails })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

/// GET /outbox/:id
pub async fn get_outbox(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut sql = "SELECT id, account_id, to_addr, subject, body, status, retries, last_error, 
                  CAST(strftime('%s', created_at) AS INTEGER) as created_at, 
                  CAST(strftime('%s', updated_at) AS INTEGER) as updated_at
                  FROM outbox WHERE id = ?".to_string();

    // Security restriction for non-Admin
    if auth_user.role != "Admin" {
        sql.push_str(" AND account_id IN (SELECT account_id FROM user_accounts WHERE user_id = ?)");
    }

    let mut q = sqlx::query_as::<_, OutboxEmail>(&sql).bind(&id);
    if auth_user.role != "Admin" {
        q = q.bind(auth_user.id);
    }

    match q.fetch_optional(&pool).await {
        Ok(Some(email)) => Json(email).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "Not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))).into_response(),
    }
}

/// POST /outbox/:id/retry
/// Resets status to 'queued' and retries to 0 so background worker will send it immediately.
pub async fn retry_outbox(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Validate ownership if not admin
    let check = if auth_user.role != "Admin" {
        sqlx::query("SELECT 1 FROM outbox WHERE id = ? AND account_id IN (SELECT account_id FROM user_accounts WHERE user_id = ?)")
            .bind(&id)
            .bind(auth_user.id)
            .fetch_optional(&pool)
            .await
    } else {
        sqlx::query("SELECT 1 FROM outbox WHERE id = ?")
            .bind(&id)
            .fetch_optional(&pool)
            .await
    };

    if check.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "ok": false, "error": "Database error checking permissions" }))).into_response();
    }

    if check.unwrap().is_none() {
        return (StatusCode::FORBIDDEN, Json(json!({ "ok": false, "error": "Access denied or not found" }))).into_response();
    }

    match sqlx::query("UPDATE outbox SET status = 'queued', retries = 0, last_error = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
    {
        Ok(_) => Json(json!({ "ok": true, "message": "Email re-queued for sending" })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

/// DELETE /outbox/:id
pub async fn delete_outbox(
    State(pool): State<SqlitePool>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Validate ownership if not admin
    let check = if auth_user.role != "Admin" {
        sqlx::query("SELECT 1 FROM outbox WHERE id = ? AND account_id IN (SELECT account_id FROM user_accounts WHERE user_id = ?)")
            .bind(&id)
            .bind(auth_user.id)
            .fetch_optional(&pool)
            .await
    } else {
        sqlx::query("SELECT 1 FROM outbox WHERE id = ?")
            .bind(&id)
            .fetch_optional(&pool)
            .await
    };

    if check.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "ok": false, "error": "Database error checking permissions" }))).into_response();
    }

    if check.unwrap().is_none() {
        return (StatusCode::FORBIDDEN, Json(json!({ "ok": false, "error": "Access denied or not found" }))).into_response();
    }

    match sqlx::query("DELETE FROM outbox WHERE id = ?")
        .bind(&id)
        .execute(&pool)
        .await
    {
        Ok(_) => Json(json!({ "ok": true, "message": "Email cancelled and removed from queue" })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}
