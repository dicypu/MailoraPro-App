pub mod jmap_proxy;
use crate::persist;
use crate::services::diff_service::AccountCreds;
use crate::services::diff_service::ACCOUNTS; // add account store
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::{http::StatusCode, Json};
use axum::{
    routing::{get, post},
    Router,
};
use serde::Deserialize; // correct import from crate root
use tower_http::services::ServeDir;

pub mod auth;
pub mod discovery;
pub mod admin;
pub mod accounts;
pub mod debug;
pub mod diff;
pub mod idle;
pub mod oauth;
pub mod sync;
pub mod test;
pub mod unified;
pub mod flags;
pub mod settings;
pub mod snooze;
pub mod contacts;
pub mod calendar;
pub mod outbox;
pub mod sheets;

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct LoginReq {
    email: String,
    password: String,
    host: Option<String>,
    port: Option<u16>,
}

async fn login(Json(payload): Json<LoginReq>) -> impl IntoResponse {
    if payload.email.is_empty() || payload.password.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let host = payload.host.unwrap_or_else(|| {
        std::env::var("IMAP_HOST").unwrap_or_else(|_| "imap.example.com".into())
    });
    let port: u16 = payload
        .port
        .or_else(|| std::env::var("IMAP_PORT").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(993);

    let res =
        crate::imap::sync::initial_snapshot(&host, port, &payload.email, &payload.password).await;
    match res {
        Ok(snap) => {
            // store creds with accountId = 1 for now
            let store = ACCOUNTS.clone();
            {
                // insert with key "1"
                let mut w = store.write().await;
                w.insert(
                    "1".into(),
                    AccountCreds {
                        email: payload.email.clone(),
                        password: payload.password.clone(),
                        host: host.to_string(),
                        port,
                    },
                );
            }
            if let Err(e) = persist::save_state().await {
                tracing::warn!("persist save error: {e}");
            }
            Json(serde_json::json!({
                "ok": true,
                "email": payload.email,
                "uidvalidity": snap.uidvalidity,
                "last_uid": snap.last_uid,
                "accountId": 1
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let error_code = if msg.contains("Application-specific password required") {
                "APP_PASSWORD_REQUIRED"
            } else if msg.contains("Invalid credentials") || msg.contains("authentication failed") {
                "INVALID_CREDENTIALS"
            } else if msg.contains("Connection reset") {
                "CONNECTION_RESET"
            } else {
                "IMAP_AUTH_FAILED"
            };
            Json(serde_json::json!({
                "ok": false,
                "error_code": error_code,
                "error": msg,
                "hint": match error_code {
                    "APP_PASSWORD_REQUIRED" => "Google Hesabında 2 Adımlı Doğrulama aç ve Uygulama Şifresi oluştur. Ardından bu şifreyi kullan.",
                    "INVALID_CREDENTIALS" => "Email / şifre veya app password yanlış.",
                    _ => "Host/port ve IMAP erişimi açık mı (Gmail ayarlarından IMAP etkin)?"
                }
            })).into_response()
        }
    }
}

async fn root_page() -> impl IntoResponse {
    axum::response::Redirect::permanent("/static/index.html")
}

async fn app_page() -> impl IntoResponse {
    Html(include_str!("../../static/index.html"))
}

use axum::extract::Json as AxumJson;
use serde::Serialize;

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct SendReq {
    accountId: String,
    to: String,
    subject: String,
    body: String,
}
#[derive(Serialize)]
struct SendResp {
    ok: bool,
}

async fn send_action(
    auth: Option<crate::rbac::AuthUser>,
    State(pool): State<sqlx::SqlitePool>,
    AxumJson(req): AxumJson<SendReq>
) -> impl IntoResponse {
    // Check if account exists (validate)
    let account_res = sqlx::query("SELECT id FROM accounts WHERE id = ?")
        .bind(&req.accountId)
        .fetch_optional(&pool)
        .await;

    if matches!(account_res, Ok(None)) {
             // Fallback for legacy "1"
             if req.accountId == "1" {
                 // allow legacy
             } else {
                 return AxumJson(serde_json::json!({"ok": false, "error": "Account not found"})).into_response();
             }
    }

    // Queue email
    match crate::services::outbox_service::queue_email(
        &pool,
        &req.accountId,
        &req.to,
        &req.subject,
        &req.body
    ).await {
        Ok(_) => {
            let user_id = auth.map(|a| a.id);
            crate::rbac::log_event(
                &pool,
                user_id,
                Some(&req.accountId),
                "SEND_QUEUED",
                &format!("Queued email to: {}", req.to)
            ).await;
            AxumJson(serde_json::json!({"ok": true, "message": "Email queued"})).into_response()
        },
        Err(e) => AxumJson(serde_json::json!({"ok": false, "error": e})).into_response(),
    }
}

pub fn routes<S>(pool: &sqlx::SqlitePool) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    sqlx::SqlitePool: axum::extract::FromRef<S>,
{
    Router::new()
        .route("/", get(root_page))
        .route("/app", get(app_page))
        .nest_service("/static", ServeDir::new("static"))
        .nest("/sheets", sheets::router())
        .route("/login", post(login))
        .route("/diff", get(diff::diff_handler))
        .route("/body", get(diff::body_handler))
        .route("/folders", get(diff::folders_handler))
        .route("/attachments", get(diff::attachments_handler))
        .route("/attachments/download", get(diff::download_attachment))
        .route("/unified/inbox", get(unified::unified_inbox))
        .route("/unified/unread", get(unified::unified_unread))
        .route("/unified/events", get(unified::unified_events))
        .route("/accounts", post(accounts::add_account))
        .route("/accounts", get(accounts::list_accounts))
        .route(
            "/accounts/:id",
            get(accounts::get_account)
                .delete(accounts::delete_account)
                .patch(accounts::patch_account),
        )
        .route("/providers", get(accounts::list_providers))
        .route("/test/connection/:account_id", get(test::test_connection))
        .route("/test/messages/:account_id", get(test::fetch_messages))
        .route("/test/smtp/:account_id", post(test::smtp_test))
        .route("/test/smtp-append/:account_id", post(test::smtp_send_and_append))
        .route("/test/sent-finalize/:account_id", get(test::sent_finalize))
        .route("/test/body/:account_id/:uid", get(test::fetch_message_body))
        .route("/test/accounts", get(test::list_test_accounts))
        .route("/test/update-append-policy/:account_id", post(test::update_append_policy))
        .route("/debug/metrics", get(test::metrics_snapshot))
        .route("/send", post(send_action))
        .route("/debug/state", get(debug::state))
        .route("/debug/probe", get(debug::probe_diff))
        .route("/sync/:account_id", post(sync::sync_account))
        .route("/sync/:account_id/:folder", post(sync::sync_folder))
        .route("/messages/:account_id", get(sync::get_messages))
        .route(
            "/messages/:account_id/:folder",
            get(sync::get_folder_messages),
        )
        .route("/messages/:account_id/:folder/:uid/flags", post(flags::update_flags))
        .route("/test/folders/:account_id", get(test::list_folders))
        .route("/settings", get(settings::get_settings))
        .route("/search", get(sync::search_messages))
        .route("/sync/:account_id/backfill-attachments", post(sync::backfill_attachments_endpoint))
        .route("/snooze/:account_id/:folder/:uid", post(snooze::snooze_message))
        .route("/unsnooze/:account_id/:folder/:uid", post(snooze::unsnooze_message))
        // ─── Outbox (Reliable Sending Queue) ─────────────────────────────────
        .route("/outbox", get(outbox::list_outbox))
        .route("/outbox/:id", get(outbox::get_outbox).delete(outbox::delete_outbox))
        .route("/outbox/:id/retry", post(outbox::retry_outbox))
        // ─── Contacts (PIM v1.6) ─────────────────────────────────────────────
        .route("/contacts", get(contacts::list_contacts).post(contacts::create_contact))
        .route("/contacts/suggest", get(contacts::suggest_contacts))
        .route("/contacts/groups", get(contacts::list_groups).post(contacts::create_group))
        .route("/contacts/import", post(contacts::import_contacts))
        .route("/contacts/export", get(contacts::export_contacts))
        .route("/contacts/:id", get(contacts::get_contact).put(contacts::update_contact).delete(contacts::delete_contact))
        .route("/contacts/:id/groups/:group_id", post(contacts::add_to_group).delete(contacts::remove_from_group))
        .route("/contacts/conflicts", get(contacts::list_conflicts))
        .route("/contacts/conflicts/:id/resolve", post(contacts::resolve_conflict))
        .route("/contacts/duplicates", get(contacts::list_duplicates))
        .route("/contacts/duplicates/merge", post(contacts::merge_duplicates))
        .route("/sync/carddav/:account_id", post(contacts::sync_carddav))
        // ─── Calendar (PIM v1.7) ─────────────────────────────────────────────
        .nest("/calendar", calendar::router().with_state(pool.clone()))
        // ─── PIM Auto-Discovery Fallback ─────────────────────────────────────
        .route("/.well-known/carddav", get(|| async { axum::response::Redirect::permanent("/static/contacts.html") }))
}
