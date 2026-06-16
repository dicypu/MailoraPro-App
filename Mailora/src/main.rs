use anyhow::Result;
use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

mod db;
mod imap;
mod models;
mod oauth;
mod persist;
mod pim;
mod rbac;
mod routes;
mod services;
mod smtp;

#[derive(Clone)]
struct AppState {
    pool: sqlx::SqlitePool,
    idle_manager: Arc<services::idle_watcher_service::IdleWatcherManager>,
    oauth_manager: Arc<oauth::OAuthManager>,
}

impl axum::extract::FromRef<AppState> for sqlx::SqlitePool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl axum::extract::FromRef<AppState> for Arc<services::idle_watcher_service::IdleWatcherManager> {
    fn from_ref(state: &AppState) -> Self {
        state.idle_manager.clone()
    }
}

impl axum::extract::FromRef<AppState> for Arc<oauth::OAuthManager> {
    fn from_ref(state: &AppState) -> Self {
        state.oauth_manager.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    // load persisted accounts
    if let Err(e) = persist::load_state().await {
        tracing::warn!("persist load error: {e}");
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,mailora_hub_imap=debug")),
        )
        .init();

    // Build a correct sqlite URL (sqlx expects sqlite://path or sqlite::memory:)
    let raw_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://mailora_imap.db".into());
    let db_url = normalize_sqlite_url(&raw_url);

    if std::path::Path::new("migrations").exists() {
        // Ensure file exists for file-based sqlite (avoid open error on some setups)
        if let Some(path) = db_file_path(&db_url) {
            if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).ok(); }
            if !path.exists() { std::fs::File::create(&path).ok(); }
        }
        let pool = sqlx::SqlitePool::connect(&db_url).await?;
        
        // Check for broken schema (legacy) and fix if needed
        if let Err(e) = db::check_and_fix_schema(&pool).await {
            tracing::warn!("schema fix failed: {e}");
        }

        if let Err(e) = db::run_migrations(&pool).await {
            // Ignore common "already exists" failures, log as info
            let msg = e.to_string();
            if msg.contains("already exists") { tracing::info!("migration benign: {msg}"); } else { tracing::warn!("migration error: {msg}"); }
        }
        if let Err(e) = db::seed_account(&pool).await {
            tracing::info!("seed skipped: {e}");
        }

        if let Err(e) = db::normalize_legacy_dates(&pool).await {
            tracing::warn!("date normalization failed: {e}");
        }

        // Create idle watcher manager
        let idle_manager = Arc::new(services::idle_watcher_service::IdleWatcherManager::new());

        // Create OAuth manager
        let oauth_manager = Arc::new(oauth::OAuthManager::new());

        let state = AppState {
            pool: pool.clone(),
            idle_manager: idle_manager.clone(),
            oauth_manager: oauth_manager.clone(),
        };

        // Start background scheduler
        crate::services::scheduler::start(pool.clone());

        // Start background maintenance service
        {
            let p = pool.clone();
            tokio::spawn(async move {
                services::maintenance_service::start_maintenance_loop(p).await;
            });
        }

        // Start background outbox service
        {
            let p = pool.clone();
            tokio::spawn(async move {
                services::outbox_service::start_outbox_loop(p).await;
            });
        }

        // Initial full sync and IDLE start on startup (non-blocking)
        {
            let pool_clone = pool.clone();
            let idle_mgr_clone = idle_manager.clone();
            tokio::spawn(async move {
                match services::account_service::list_accounts(&pool_clone).await {
                    Ok(accts) => {
                        for acc in accts {
                            if !acc.enabled { continue; }
                            
                            // Start IDLE watcher immediately
                            if acc.provider != crate::models::account::EmailProvider::Gmail { // Gmail IDLE can be tricky, but let's try or skip if unsure. Standard implementations usually support it. Let's enable for all except maybe complex oauth if not ready?
                                // Actually let's just try to start it.
                                let mgr = idle_mgr_clone.clone();
                                let acc_for_idle = acc.clone();
                                tokio::spawn(async move {
                                    // Decrypt password if needed
                                    let mut final_acc = acc_for_idle.clone();
                                    if final_acc.password.is_empty() {
                                         if let Ok(a) = final_acc.clone().with_password() { final_acc = a; }
                                    }
                                    if !final_acc.password.is_empty() {
                                        if let Err(e) = mgr.start_watcher(final_acc).await {
                                            tracing::warn!("Failed to auto-start IDLE for {}: {}", acc_for_idle.email, e);
                                        }
                                    }
                                });
                            }

                            // Skip Gmail sync for now (legacy logic)
                            if matches!(acc.provider, crate::models::account::EmailProvider::Gmail) { continue; }
                            
                            let mut acc_dec = acc.clone();
                            if acc_dec.password.is_empty() {
                                if let Ok(a) = acc_dec.clone().with_password() { acc_dec = a; } else { continue; }
                            }
                            if acc_dec.password.is_empty() { continue; }
                            
                            let p = pool_clone.clone();
                            tokio::spawn(async move {
                                // Use sync_account (which now reuses session)
                                match services::message_sync_service::sync_account_messages(&p, &acc_dec).await {
                                    Ok(stats) => {
                                        tracing::info!(email=%acc_dec.email, folders=%stats.len(), "initial sync completed");
                                        let _ = services::account_service::update_last_sync(&p, &acc_dec.id).await;
                                    }
                                    Err(e) => tracing::warn!(email=%acc_dec.email, error=%e.to_string(), "initial sync failed"),
                                }
                            });
                        }
                    }
                    Err(e) => tracing::warn!("initial sync: list_accounts failed: {e}"),
                }
            });
        }

        let idle_routes: Router<AppState> = Router::new()
            .route(
                "/idle/start/:account_id",
                axum::routing::post(routes::idle::start_idle_watcher),
            )
            .route(
                "/idle/stop/:account_id",
                axum::routing::post(routes::idle::stop_idle_watcher),
            )
            .route("/idle/status", get(routes::idle::idle_status))
            .route("/idle/events", get(routes::idle::idle_events_stream));

        let oauth_routes = Router::new()
            .route("/oauth/start", get(routes::oauth::start_oauth))
            .route("/oauth/callback", get(routes::oauth::oauth_callback))
            .route("/oauth/setup-guide", get(routes::oauth::oauth_setup_guide))
            .with_state(oauth_manager.clone());

        let app = Router::new()
            .route("/healthz", get(|| async { "ok" }))
            .merge(routes::routes(&pool))
            .merge(routes::auth::router().with_state(state.pool.clone()))
            .merge(routes::admin::router().with_state(state.pool.clone()))
            .merge(routes::discovery::router())
            .merge(idle_routes)
            .merge(oauth_routes)
            // App state
            .with_state(state.clone());

        let port: u16 = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3030); // Stalwart 8787'de, biz 3030'da çalışalım

        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("listening on http://{}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    } else {
        tracing::warn!("migrations folder not found, skipping DB setup");
    }
    // save state after shutdown
    if let Err(e) = persist::save_state().await {
        tracing::warn!("persist save error: {e}");
    }
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;
    let ctrl_c = async {
        signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}

fn normalize_sqlite_url(input: &str) -> String {
    // Accept forms: sqlite:foo.db (fix), sqlite://foo.db (ok), file:foo.db (convert), just path (prepend)
    if input.starts_with("sqlite://") || input.starts_with("sqlite::memory:") {
        return input.to_string();
    }
    if input.starts_with("sqlite:") {
        // single colon like sqlite:foo.db -> make it sqlite://foo.db
        let rest = input.trim_start_matches("sqlite:");
        return format!("sqlite://{}", rest.trim_start_matches('/'));
    }
    if input.starts_with("file:") {
        return format!("sqlite://{}", input.trim_start_matches("file:"));
    }
    // bare path
    format!("sqlite://{}", input)
}

fn db_file_path(url: &str) -> Option<std::path::PathBuf> {
    // sqlite URLs: sqlite://<path>. Strip prefix
    if let Some(rest) = url.strip_prefix("sqlite://") {
        if rest == ":memory:" {
            return None;
        }
        return Some(std::path::PathBuf::from(rest));
    }
    None
}

// index.html serve için routes/mod.rs içinde root_page kullanılıyor.
