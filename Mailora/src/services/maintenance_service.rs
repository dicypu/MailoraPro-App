use sqlx::SqlitePool;
use tokio::time::{sleep, Duration};

/// Starts the background maintenance loop
pub async fn start_maintenance_loop(pool: SqlitePool) {
    tracing::info!("Starting database maintenance service (Vacuum/Analyze)");

    // Initial delay to let the app start up comfortably
    sleep(Duration::from_secs(60)).await;

    loop {
        if let Err(e) = perform_maintenance(&pool).await {
            tracing::error!("Maintenance job failed: {}", e);
        }
        
        // Schedule next run in 24 hours
        sleep(Duration::from_secs(86400)).await;
    }
}

async fn perform_maintenance(pool: &SqlitePool) -> anyhow::Result<()> {
    tracing::info!("Running automated database maintenance...");
    
    // Optimize database
    sqlx::query("PRAGMA optimize").execute(pool).await?;
    
    // Periodically run VACUUM (can be expensive, maybe only if needed?)
    // For now, let's run it. In WAL mode it shouldn't block readers, but writers might block.
    // It's safe to separate.
    sqlx::query("VACUUM").execute(pool).await?;
    
    // Analyze for query planner
    sqlx::query("ANALYZE").execute(pool).await?;

    tracing::info!("Database maintenance completed successfully.");
    Ok(())
}
