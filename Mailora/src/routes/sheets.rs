use axum::{
    extract::{State, Json, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use axum::http::HeaderMap;

#[derive(Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
}

#[derive(Serialize)]
pub struct SpreadsheetListItem {
    pub id: String,
    pub name: String,
    pub owner_id: i64,
    pub is_owner: bool,
    pub shared_with: String,
}

#[derive(Serialize, Deserialize)]
pub struct SpreadsheetData {
    pub id: String,
    pub owner_id: Option<i64>,
    pub name: String,
    pub data: String, // Stringified JSON of data, sheets, activeSheet
    pub shared_with: String, // JSON array of user_ids or "ALL"
}

#[derive(Serialize)]
pub struct GenericRes<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

fn extract_user_id(headers: &HeaderMap) -> Option<i64> {
    if let Some(auth_val) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_val.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                if let Some(colon_idx) = token.find(':') {
                    if let Ok(id) = token[..colon_idx].parse::<i64>() {
                        return Some(id);
                    }
                }
            }
        }
    }
    None
}

async fn list_users(
    State(pool): State<SqlitePool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _user_id = match extract_user_id(&headers) {
        Some(id) => id,
        None => return (StatusCode::UNAUTHORIZED, Json(GenericRes::<Vec<UserInfo>> { ok: false, data: None, error: Some("Unauthorized".into()) })).into_response(),
    };

    let users = sqlx::query_as!(UserInfo, "SELECT id as \"id!\", username as \"username!\" FROM users")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    (StatusCode::OK, Json(GenericRes { ok: true, data: Some(users), error: None })).into_response()
}

async fn list_sheets(
    State(pool): State<SqlitePool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers) {
        Some(id) => id,
        None => return (StatusCode::UNAUTHORIZED, Json(GenericRes::<Vec<SpreadsheetListItem>> { ok: false, data: None, error: Some("Unauthorized".into()) })).into_response(),
    };

    let sql = r#"
        SELECT id, name, owner_id, shared_with
        FROM spreadsheets
        WHERE owner_id = ? 
           OR shared_with = '"ALL"'
           OR shared_with LIKE ?
    "#;

    let p1 = format!("%[{}]%", user_id);
    let p2 = format!("%[{}%", user_id);
    let p3 = format!("% {} %", user_id);
    let p4 = format!("%{}]%", user_id);

    let rows = sqlx::query!(
        r#"
        SELECT id, name, owner_id, shared_with
        FROM spreadsheets
        WHERE owner_id = ? 
           OR shared_with = '"ALL"'
           OR shared_with LIKE ?
           OR shared_with LIKE ?
           OR shared_with LIKE ?
        "#,
        user_id,
        p2,
        p3,
        p4
    )
    .fetch_all(&pool)
    .await;

    // Actually, SQLite doesn't have great JSON support enabled by default in all builds, but we can just fetch all and filter in memory if needed, or use a better approach.
    // Let's do a fetch_all and filter in Rust to be 100% safe since user base is small.

    let all_sheets = sqlx::query!("SELECT id as \"id!\", name as \"name!\", owner_id as \"owner_id!\", shared_with as \"shared_with!\" FROM spreadsheets")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let mut result = Vec::new();
    for sheet in all_sheets {
        let mut has_access = sheet.owner_id == user_id;
        if !has_access {
            if sheet.shared_with == "\"ALL\"" || sheet.shared_with == "ALL" {
                has_access = true;
            } else if let Ok(shared_list) = serde_json::from_str::<Vec<i64>>(&sheet.shared_with) {
                if shared_list.contains(&user_id) {
                    has_access = true;
                }
            }
        }

        if has_access {
            result.push(SpreadsheetListItem {
                id: sheet.id.clone(),
                name: sheet.name.clone(),
                owner_id: sheet.owner_id,
                is_owner: sheet.owner_id == user_id,
                shared_with: sheet.shared_with,
            });
        }
    }

    (StatusCode::OK, Json(GenericRes::<Vec<SpreadsheetListItem>> { ok: true, data: Some(result), error: None })).into_response()
}

async fn get_sheet(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers) {
        Some(id) => id,
        None => return (StatusCode::UNAUTHORIZED, Json(GenericRes::<SpreadsheetData> { ok: false, data: None, error: Some("Unauthorized".into()) })).into_response(),
    };

    let sheet = sqlx::query!("SELECT id as \"id!\", owner_id as \"owner_id!\", name as \"name!\", data as \"data!\", shared_with as \"shared_with!\" FROM spreadsheets WHERE id = ?", id)
        .fetch_optional(&pool)
        .await;

    match sheet {
        Ok(Some(s)) => {
            let mut has_access = s.owner_id == user_id;
            if !has_access {
                if s.shared_with == "\"ALL\"" || s.shared_with == "ALL" {
                    has_access = true;
                } else if let Ok(shared_list) = serde_json::from_str::<Vec<i64>>(&s.shared_with) {
                    if shared_list.contains(&user_id) {
                        has_access = true;
                    }
                }
            }

            if has_access {
                let data = SpreadsheetData {
                    id: s.id,
                    owner_id: Some(s.owner_id),
                    name: s.name,
                    data: s.data,
                    shared_with: s.shared_with,
                };
                (StatusCode::OK, Json(GenericRes::<SpreadsheetData> { ok: true, data: Some(data), error: None })).into_response()
            } else {
                (StatusCode::FORBIDDEN, Json(GenericRes::<SpreadsheetData> { ok: false, data: None, error: Some("Access denied".into()) })).into_response()
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(GenericRes::<SpreadsheetData> { ok: false, data: None, error: Some("Not found".into()) })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(GenericRes::<SpreadsheetData> { ok: false, data: None, error: Some(e.to_string()) })).into_response(),
    }
}

async fn save_sheet(
    State(pool): State<SqlitePool>,
    headers: HeaderMap,
    Json(payload): Json<SpreadsheetData>,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers) {
        Some(id) => id,
        None => return (StatusCode::UNAUTHORIZED, Json(GenericRes::<()> { ok: false, data: None, error: Some("Unauthorized".into()) })).into_response(),
    };

    // Check if exists and check permissions
    let existing = sqlx::query!("SELECT owner_id as \"owner_id!\" FROM spreadsheets WHERE id = ?", payload.id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    if let Some(record) = existing {
        // Only owner can update shared_with settings, others can just update data
        // For simplicity, anyone with access can edit data
        let mut has_access = record.owner_id == user_id;
        
        let shared_with = if record.owner_id == user_id {
            // Owner can update everything including shared_with
            payload.shared_with.clone()
        } else {
            // Non-owner cannot update shared_with
            let sheet = sqlx::query!("SELECT shared_with as \"shared_with!\" FROM spreadsheets WHERE id = ?", payload.id).fetch_one(&pool).await.unwrap();
            
            if sheet.shared_with == "\"ALL\"" || sheet.shared_with == "ALL" {
                has_access = true;
            } else if let Ok(shared_list) = serde_json::from_str::<Vec<i64>>(&sheet.shared_with) {
                if shared_list.contains(&user_id) {
                    has_access = true;
                }
            }
            sheet.shared_with
        };

        if !has_access {
            return (StatusCode::FORBIDDEN, Json(GenericRes::<()> { ok: false, data: None, error: Some("Access denied".into()) })).into_response();
        }

        let sql = "UPDATE spreadsheets SET name = ?, data = ?, shared_with = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?";
        match sqlx::query(sql)
            .bind(&payload.name)
            .bind(&payload.data)
            .bind(&shared_with)
            .bind(&payload.id)
            .execute(&pool)
            .await 
        {
            Ok(_) => (StatusCode::OK, Json(GenericRes::<()> { ok: true, data: None, error: None })).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(GenericRes::<()> { ok: false, data: None, error: Some(e.to_string()) })).into_response(),
        }

    } else {
        // Create new
        let sql = "INSERT INTO spreadsheets (id, owner_id, name, data, shared_with, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)";
        match sqlx::query(sql)
            .bind(&payload.id)
            .bind(user_id)
            .bind(&payload.name)
            .bind(&payload.data)
            .bind(&payload.shared_with)
            .execute(&pool)
            .await 
        {
            Ok(_) => (StatusCode::OK, Json(GenericRes::<()> { ok: true, data: None, error: None })).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(GenericRes::<()> { ok: false, data: None, error: Some(e.to_string()) })).into_response(),
        }
    }
}

async fn delete_sheet(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_id(&headers) {
        Some(id) => id,
        None => return (StatusCode::UNAUTHORIZED, Json(GenericRes::<()> { ok: false, data: None, error: Some("Unauthorized".into()) })).into_response(),
    };

    let existing = sqlx::query!("SELECT owner_id as \"owner_id!\" FROM spreadsheets WHERE id = ?", id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    if let Some(record) = existing {
        if record.owner_id != user_id {
            return (StatusCode::FORBIDDEN, Json(GenericRes::<()> { ok: false, data: None, error: Some("Only owner can delete".into()) })).into_response();
        }

        match sqlx::query!("DELETE FROM spreadsheets WHERE id = ?", id)
            .execute(&pool)
            .await 
        {
            Ok(_) => (StatusCode::OK, Json(GenericRes::<()> { ok: true, data: None, error: None })).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(GenericRes::<()> { ok: false, data: None, error: Some(e.to_string()) })).into_response(),
        }
    } else {
        (StatusCode::NOT_FOUND, Json(GenericRes::<()> { ok: false, data: None, error: Some("Not found".into()) })).into_response()
    }
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    SqlitePool: axum::extract::FromRef<S>,
{
    Router::new()
        .route("/users", get(list_users))
        .route("/", get(list_sheets).post(save_sheet))
        .route("/:id", get(get_sheet).delete(delete_sheet))
}
