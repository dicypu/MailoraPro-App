use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(login))
            .route("/register", web::post().to(register))
    )
    .service(
        web::scope("/api")
            .route("/accounts", web::get().to(get_accounts))
            .route("/messages", web::get().to(get_messages))
            .route("/messages/{id}", web::get().to(get_message))
            .route("/send", web::post().to(send_message))
            .route("/folders", web::get().to(get_folders))
            .route("/settings", web::get().to(get_settings))
    );
}

#[derive(Deserialize)]
struct LoginReq { username: String, password: String }
#[derive(Serialize)]
struct LoginRes { token: String, username: String, role: String }
#[derive(Deserialize)]
struct RegisterReq { username: String, password: String }

async fn login(body: web::Json<LoginReq>) -> HttpResponse {
    // TODO: Real auth with bcrypt + JWT
    // For now, accept any login
    let token = format!("jwt_token_{}", uuid::Uuid::new_v4());
    HttpResponse::Ok().json(LoginRes {
        token, username: body.username.clone(), role: "Admin".to_string(),
    })
}

async fn register(body: web::Json<RegisterReq>) -> HttpResponse {
    // TODO: Real registration with DB
    if body.username.len() < 3 || body.password.len() < 6 {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Username min 3, password min 6 chars"}));
    }
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

async fn get_accounts() -> HttpResponse {
    // TODO: Real accounts from DB
    HttpResponse::Ok().json(serde_json::json!([]))
}

async fn get_messages() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!([]))
}

async fn get_message(path: web::Path<String>) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"id": path.into_inner()}))
}

async fn send_message(req: HttpRequest) -> HttpResponse {
    // Content-Length is already checked by PayloadConfig middleware
    // If we reach here, the payload is under 10MB
    log::info!("Send message request received");
    HttpResponse::Ok().json(serde_json::json!({"success": true, "id": uuid::Uuid::new_v4().to_string()}))
}

async fn get_folders() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!(["Inbox","Sent","Drafts","Spam","Trash"]))
}

async fn get_settings() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "database": {"url": "sqlite://mailora.db"},
        "imap": {"server": "imap.gmail.com", "port": 993},
        "smtp": {"server": "smtp.gmail.com", "port": 587},
        "security": {"sanitize_html": true},
        "features": {"attachments": true, "unified_view": true}
    }))
}
