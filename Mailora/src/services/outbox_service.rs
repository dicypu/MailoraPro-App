use crate::models::{account::Account, outbox::OutboxEmail};
use crate::services::account_service;
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::time::sleep;

/// Queue an email to be sent
pub async fn queue_email(
    pool: &SqlitePool,
    account_id: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO outbox (id, account_id, to_addr, subject, body) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(account_id)
    .bind(to)
    .bind(subject)
    .bind(body)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(id)
}

/// Background loop to process outbox
pub async fn start_outbox_loop(pool: SqlitePool) {
    tracing::info!("Starting Outbox Service loop...");
    loop {
        if let Err(e) = process_batch(&pool).await {
            tracing::error!("Outbox processing error: {}", e);
        }
        sleep(Duration::from_secs(10)).await;
    }
}

async fn process_batch(pool: &SqlitePool) -> Result<(), anyhow::Error> {
    // Select queued or failed (with retries < 3)
    let emails = sqlx::query_as::<_, OutboxEmail>(
        "SELECT id, account_id, to_addr, subject, body, status, retries, last_error, 
         strftime('%s', created_at) as created_at, strftime('%s', updated_at) as updated_at
         FROM outbox 
         WHERE status = 'queued' OR (status = 'failed' AND retries < 3)
         ORDER BY created_at ASC
         LIMIT 5"
    )
    .fetch_all(pool)
    .await?;

    if emails.is_empty() {
        return Ok(());
    }

    tracing::info!("Outbox: Processing {} emails", emails.len());

    for email in emails {
        // Mark as processing
        sqlx::query("UPDATE outbox SET status = 'processing', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&email.id)
            .execute(pool)
            .await?;

        // Validate account
        let account_opt = account_service::get_account(pool, &email.account_id).await?;
        match account_opt {
            Some(account) => {
                match send_via_smtp(&account, &email.to_addr, &email.subject, &email.body).await {
                    Ok(_) => {
                        tracing::info!("Outbox: Email {} sent successfully", email.id);
                        sqlx::query("UPDATE outbox SET status = 'sent', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                            .bind(&email.id)
                            .execute(pool)
                            .await?;
                    }
                    Err(e) => {
                        tracing::error!("Outbox: Failed to send {}: {}", email.id, e);
                        sqlx::query(
                            "UPDATE outbox SET status = 'failed', retries = retries + 1, last_error = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
                        )
                        .bind(e.to_string())
                        .bind(&email.id)
                        .execute(pool)
                        .await?;
                    }
                }
            }
            None => {
                 tracing::error!("Outbox: Account {} not found for email {}", email.account_id, email.id);
                 sqlx::query("UPDATE outbox SET status = 'failed', last_error = 'Account not found', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(&email.id)
                    .execute(pool)
                    .await?;
            }
        }
    }

    Ok(())
}

async fn send_via_smtp(account: &Account, to: &str, subject: &str, body: &str) -> Result<(), anyhow::Error> {
    use lettre::{
        transport::smtp::authentication::Credentials,
        transport::smtp::client::Tls,
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    };

    let creds = Credentials::new(account.email.clone(), account.password.clone());
    
    // Tls logic (simplified from action.rs)
    let tls = if account.smtp_port == 465 {
        Tls::Wrapper(lettre::transport::smtp::client::TlsParameters::new(
            account.smtp_host.clone(),
        )?)
    } else {
        Tls::Required(lettre::transport::smtp::client::TlsParameters::new(
            account.smtp_host.clone(),
        )?)
    };

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&account.smtp_host)?
        .credentials(creds)
        .port(account.smtp_port)
        .tls(tls)
        .build();

    let email = Message::builder()
        .from(account.email.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .body(body.to_string())?;

    let mailer_result = tokio::time::timeout(std::time::Duration::from_secs(15), mailer.send(email)).await;
    match mailer_result {
        Ok(result) => {
            result?;
            Ok(())
        }
        Err(_) => {
            Err(anyhow::anyhow!("SMTP connection timed out after 15 seconds"))
        }
    }
}
