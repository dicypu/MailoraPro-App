use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, delete},
    Json, Router,
};
use crate::models::user::User;
use crate::rbac::AdminUser;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct EventLog {
    pub id: i64,
    pub user_id: Option<i64>,
    pub account_id: Option<String>,
    pub action: String,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct AdminStats {
    pub total_emails: i64,
    pub total_users: i64,
    pub spam_blocked: i64,
    pub error_count: i64,
    pub recent_logs: Vec<EventLog>,
}

#[derive(Deserialize)]
pub struct UpdateRoleReq {
    pub role: String,
}

#[derive(Deserialize)]
pub struct AssignAccountReq {
    pub account_id: String,
}

#[derive(Serialize)]
pub struct UserWithAccounts {
    pub user: User,
    pub accounts: Vec<String>,
}

async fn list_users(
    _admin: AdminUser,
    State(pool): State<sqlx::SqlitePool>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&pool)
        .await {
            Ok(users) => (StatusCode::OK, Json(users)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        }
}

async fn update_user_role(
    _admin: AdminUser,
    State(pool): State<sqlx::SqlitePool>,
    Path(user_id): Path<i64>,
    Json(req): Json<UpdateRoleReq>,
) -> impl IntoResponse {
    match sqlx::query("UPDATE users SET role = ? WHERE id = ?")
        .bind(&req.role)
        .bind(user_id)
        .execute(&pool)
        .await {
            Ok(_) => StatusCode::OK.into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        }
}

async fn assign_account(
    _admin: AdminUser,
    State(pool): State<sqlx::SqlitePool>,
    Path(user_id): Path<i64>,
    Json(req): Json<AssignAccountReq>,
) -> impl IntoResponse {
    match sqlx::query("INSERT OR IGNORE INTO user_accounts (user_id, account_id) VALUES (?, ?)")
        .bind(user_id)
        .bind(&req.account_id)
        .execute(&pool)
        .await {
            Ok(_) => StatusCode::CREATED.into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        }
}

async fn unassign_account(
    _admin: AdminUser,
    State(pool): State<sqlx::SqlitePool>,
    Path((user_id, account_id)): Path<(i64, String)>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM user_accounts WHERE user_id = ? AND account_id = ?")
        .bind(user_id)
        .bind(&account_id)
        .execute(&pool)
        .await {
            Ok(_) => StatusCode::OK.into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        }
}

async fn list_user_accounts(
    _admin: AdminUser,
    State(pool): State<sqlx::SqlitePool>,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    match sqlx::query_scalar::<_, String>("SELECT account_id FROM user_accounts WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(&pool)
        .await {
            Ok(accounts) => (StatusCode::OK, Json(accounts)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        }
}

async fn delete_user(
    _admin: AdminUser,
    State(pool): State<sqlx::SqlitePool>,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    // Also delete from user_accounts to maintain consistency
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Tx begin failed: {}", e)}))).into_response(),
    };

    let _ = sqlx::query("DELETE FROM user_accounts WHERE user_id = ?")
        .bind(user_id)
        .execute(&mut *tx)
        .await;

    match sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&mut *tx)
        .await {
            Ok(_) => {
                let _ = tx.commit().await;
                StatusCode::OK.into_response()
            },
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
        }
}

async fn get_admin_stats(
    _admin: AdminUser,
    State(pool): State<sqlx::SqlitePool>,
) -> impl IntoResponse {
    let total_emails: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages").fetch_one(&pool).await.unwrap_or(0);
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap_or(0);
    let spam_blocked: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE folder = 'Spam'").fetch_one(&pool).await.unwrap_or(0);
    let error_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_logs WHERE action LIKE '%ERROR%' OR details LIKE '%error%'").fetch_one(&pool).await.unwrap_or(0);

    let recent_logs: Vec<EventLog> = sqlx::query_as!(
        EventLog,
        "SELECT id, user_id, account_id, action, details, ip_address, CAST(created_at AS TEXT) as created_at FROM event_logs ORDER BY created_at DESC LIMIT 50"
    ).fetch_all(&pool).await.unwrap_or_default();

    let stats = AdminStats {
        total_emails,
        total_users,
        spam_blocked,
        error_count,
        recent_logs,
    };

    (StatusCode::OK, Json(stats)).into_response()
}

pub fn router() -> Router<sqlx::SqlitePool> {
    Router::new()
        .route("/admin/stats", get(get_admin_stats))
        .route("/admin/users", get(list_users))
        .route("/admin/users/:user_id", delete(delete_user))
        .route("/admin/users/:user_id/role", patch(update_user_role))
        .route("/admin/users/:user_id/accounts", get(list_user_accounts).post(assign_account))
        .route("/admin/users/:user_id/accounts/:account_id", delete(unassign_account))
}
