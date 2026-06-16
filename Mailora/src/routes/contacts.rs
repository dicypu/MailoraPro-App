use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use sqlx::SqlitePool;

use crate::{
    models::contact::{ContactQuery, ContactRequest},
    services::{carddav_service, contact_service},
};

/// POST /sync/carddav/:account_id — manual CardDAV sync trigger
pub async fn sync_carddav(
    State(pool): State<SqlitePool>,
    Path(account_id): Path<String>,
) -> impl IntoResponse {
    match carddav_service::sync_account(&pool, &account_id).await {
        Ok(result) => Json(serde_json::json!({ "ok": true, "result": result })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

/// GET /contacts
pub async fn list_contacts(
    State(pool): State<SqlitePool>,
    Query(q): Query<ContactQuery>,
) -> impl IntoResponse {
    match contact_service::list_contacts(&pool, &q).await {
        Ok(contacts) => Json(serde_json::json!({ "contacts": contacts, "count": contacts.len() })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

/// GET /contacts/suggest?q=...
pub async fn suggest_contacts(
    State(pool): State<SqlitePool>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let q = params.get("q").map(|s| s.as_str()).unwrap_or("");
    let account_id = params.get("account_id").map(|s| s.as_str());
    match contact_service::suggest_contacts(&pool, q, account_id).await {
        Ok(suggestions) => Json(serde_json::json!({ "suggestions": suggestions })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

/// GET /contacts/:id
pub async fn get_contact(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match contact_service::get_contact(&pool, &id).await {
        Ok(Some(c)) => Json(c).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

/// POST /contacts
pub async fn create_contact(
    State(pool): State<SqlitePool>,
    Json(req): Json<ContactRequest>,
) -> impl IntoResponse {
    match contact_service::create_contact(&pool, req).await {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({ "ok": true, "id": id }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

/// PUT /contacts/:id
pub async fn update_contact(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Json(req): Json<ContactRequest>,
) -> impl IntoResponse {
    match contact_service::update_contact(&pool, &id, req).await {
        Ok(true)  => Json(serde_json::json!({ "ok": true })).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "ok": false, "error": "Not found" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

/// DELETE /contacts/:id
pub async fn delete_contact(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match contact_service::delete_contact(&pool, &id).await {
        Ok(true)  => Json(serde_json::json!({ "ok": true })).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "ok": false }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

// ── Groups ────────────────────────────────────────────────────

/// GET /contacts/groups?account_id=...
pub async fn list_groups(
    State(pool): State<SqlitePool>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let Some(account_id) = params.get("account_id") else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "account_id required" }))).into_response();
    };
    match contact_service::list_groups(&pool, account_id).await {
        Ok(groups) => Json(serde_json::json!({ "groups": groups })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

/// POST /contacts/groups
pub async fn create_group(
    State(pool): State<SqlitePool>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let Some(account_id) = body.get("account_id").and_then(|v| v.as_str()) else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "account_id required" }))).into_response();
    };
    let Some(name) = body.get("name").and_then(|v| v.as_str()) else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "name required" }))).into_response();
    };
    let color = body.get("color").and_then(|v| v.as_str());
    match contact_service::create_group(&pool, account_id, name, color).await {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({ "ok": true, "id": id }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

/// POST /contacts/:id/groups/:group_id
pub async fn add_to_group(
    State(pool): State<SqlitePool>,
    Path((contact_id, group_id)): Path<(String, String)>,
) -> impl IntoResponse {
    match contact_service::add_to_group(&pool, &contact_id, &group_id).await {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

/// DELETE /contacts/:id/groups/:group_id
pub async fn remove_from_group(
    State(pool): State<SqlitePool>,
    Path((contact_id, group_id)): Path<(String, String)>,
) -> impl IntoResponse {
    match contact_service::remove_from_group(&pool, &contact_id, &group_id).await {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

/// POST /contacts/import?account_id=...
/// Body: raw .vcf content (Content-Type: text/vcard or text/plain)
pub async fn import_contacts(
    State(pool): State<SqlitePool>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> impl IntoResponse {
    let Some(account_id) = params.get("account_id").cloned() else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "account_id required" }))).into_response();
    };

    let vcf_content = String::from_utf8_lossy(&body).to_string();
    if vcf_content.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "No file data" }))).into_response();
    }

    match contact_service::import_vcf(&pool, &account_id, &vcf_content).await {
        Ok((inserted, updated, skipped)) => Json(serde_json::json!({
            "ok": true,
            "inserted": inserted,
            "updated": updated,
            "skipped": skipped
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "ok": false, "error": e.to_string() }))).into_response(),
    }
}

/// GET /contacts/export?account_id=...&group_id=...
pub async fn export_contacts(
    State(pool): State<SqlitePool>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let Some(account_id) = params.get("account_id") else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "account_id required" }))).into_response();
    };
    let group_id = params.get("group_id").map(|s| s.as_str());

    match contact_service::export_vcf(&pool, account_id, group_id).await {
        Ok(vcf) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/vcard; charset=utf-8"),
                (header::CONTENT_DISPOSITION, "attachment; filename=\"contacts.vcf\""),
            ],
            vcf,
        ).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

// ── Conflicts & Duplicates ────────────────────────────────────

pub async fn list_conflicts(
    State(pool): State<SqlitePool>,
    Query(params): Query<ContactQuery>,
) -> impl IntoResponse {
    let Some(account_id) = params.account_id else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "account_id required" }))).into_response();
    };
    match contact_service::list_conflicts(&pool, &account_id).await {
        Ok(c) => Json(c).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

pub async fn resolve_conflict(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Query(params): Query<ContactQuery>,
    Json(payload): Json<crate::models::contact::ResolveConflictRequest>,
) -> impl IntoResponse {
    let Some(account_id) = params.account_id else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "account_id required" }))).into_response();
    };
    match contact_service::resolve_conflict(&pool, &account_id, &id, payload).await {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

pub async fn list_duplicates(
    State(pool): State<SqlitePool>,
    Query(params): Query<ContactQuery>,
) -> impl IntoResponse {
    let Some(account_id) = params.account_id else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "account_id required" }))).into_response();
    };
    match contact_service::find_duplicates(&pool, &account_id).await {
        Ok(d) => Json(d).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}

pub async fn merge_duplicates(
    State(pool): State<SqlitePool>,
    Json(payload): Json<crate::models::contact::MergeDuplicatesRequest>,
) -> impl IntoResponse {
    let account_id = payload.account_id.clone();
    match contact_service::merge_duplicates(&pool, &account_id, payload).await {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}
