use axum::{Json, response::IntoResponse};
use serde::Serialize;
use std::env;

#[derive(Serialize)]
struct SettingsResponse {
    database: DatabaseSettings,
    imap: ImapSettings,
    smtp: SmtpSettings,
    oauth: OAuthSettings,
    security: SecuritySettings,
    features: FeatureSettings,
    telemetry: TelemetrySettings,
}

#[derive(Serialize)] struct DatabaseSettings { url: String }
#[derive(Serialize)] struct ImapSettings { server: String, port: u16 }
#[derive(Serialize)] struct SmtpSettings { server: String, port: u16, username: String }
#[derive(Serialize)] struct OAuthSettings { redirect_uri: String, providers: Vec<String> }
#[derive(Serialize)] struct SecuritySettings { sanitize_html: bool }
#[derive(Serialize)] struct FeatureSettings { attachments: bool, unified_view: bool }
#[derive(Serialize)] struct TelemetrySettings { log_level: String }

pub async fn get_settings() -> impl IntoResponse {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://mailora_imap.db".into());
    let imap_server = env::var("IMAP_SERVER").or_else(|_| env::var("IMAP_HOST")).unwrap_or_else(|_| "imap.example.com".into());
    let imap_port: u16 = env::var("IMAP_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(993);
    let smtp_server = env::var("SMTP_SERVER").or_else(|_| env::var("SMTP_HOST")).unwrap_or_else(|_| "smtp.example.com".into());
    let smtp_port: u16 = env::var("SMTP_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(587);
    let smtp_username = env::var("SMTP_USERNAME").unwrap_or_else(|_| "".into());
    let redirect_uri = env::var("OAUTH_REDIRECT_URI").unwrap_or_else(|_| "http://localhost:3030/oauth/callback".into());
    let providers = vec!["gmail".into(), "outlook".into(), "yahoo".into(), "icloud".into(), "custom".into()];
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".into());

    let resp = SettingsResponse {
        database: DatabaseSettings { url: database_url },
        imap: ImapSettings { server: imap_server, port: imap_port },
        smtp: SmtpSettings { server: smtp_server, port: smtp_port, username: smtp_username },
        oauth: OAuthSettings { redirect_uri, providers },
        security: SecuritySettings { sanitize_html: true },
        features: FeatureSettings { attachments: true, unified_view: true },
        telemetry: TelemetrySettings { log_level },
    };
    Json(resp)
}
