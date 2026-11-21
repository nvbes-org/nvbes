use chrono::{Duration, Utc};
use nvbes_email::{
    EmailCategory, EmailClient, EmailClientConfig, EmailClientError, EmailCommand,
    EmailIdempotencyKey, EmailReceipt, EmailRecipient, EmailRequestContext, EmailTemplate,
};
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;

const RECIPIENT_ENV: &str = "NVBES_EMAIL_SYNTHETIC_RECIPIENT";
const RUN_ID_ENV: &str = "NVBES_EMAIL_SYNTHETIC_RUN_ID";
const MAX_COLD_START_ATTEMPTS: u8 = 5;
const COLD_START_RETRY_SECONDS: [u64; 4] = [1, 2, 4, 8];
const DELIVERY_POLL_ATTEMPTS: u8 = 60;
const DELIVERY_POLL_SECONDS: u64 = 5;

#[derive(Serialize)]
struct SyntheticSmokeReceipt {
    message_id: String,
    accepted_at: chrono::DateTime<Utc>,
    deliver_before: chrono::DateTime<Utc>,
    duplicate: bool,
    attempts: u8,
    delivery_state: String,
    processed_provider_events: i64,
}

pub async fn run() -> anyhow::Result<()> {
    let recipient = required_env(RECIPIENT_ENV)?;
    let run_id = required_env(RUN_ID_ENV)?;
    let command = command(&recipient, &run_id, Utc::now())?;
    let config = EmailClientConfig::from_env("production")?;
    let (receipt, attempts) = submit_after_cold_start(&config, &command).await?;
    let (delivery_state, processed_provider_events) =
        wait_for_signed_delivery(&receipt.message_id).await?;

    println!(
        "{}",
        serde_json::to_string(&SyntheticSmokeReceipt {
            message_id: receipt.message_id,
            accepted_at: receipt.accepted_at,
            deliver_before: receipt.deliver_before,
            duplicate: receipt.duplicate,
            attempts,
            delivery_state,
            processed_provider_events,
        })?
    );
    Ok(())
}

async fn wait_for_signed_delivery(message_id: &str) -> anyhow::Result<(String, i64)> {
    let database_url = required_env("NVBES_EMAIL_DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&database_url)
        .await?;
    let result = wait_for_signed_delivery_with_pool(&pool, message_id).await;
    pool.close().await;
    result
}

async fn wait_for_signed_delivery_with_pool(
    pool: &sqlx::PgPool,
    message_id: &str,
) -> anyhow::Result<(String, i64)> {
    for _attempt in 1..=DELIVERY_POLL_ATTEMPTS {
        let delivery = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT message.state::text,
                   COUNT(event.id) FILTER (
                       WHERE event.processed_at IS NOT NULL
                         AND event.processing_result = 'state_applied'
                   )
            FROM email_messages AS message
            LEFT JOIN email_provider_events AS event
              ON event.provider_message_id = message.provider_message_id
            WHERE message.message_id = $1
            GROUP BY message.id
            "#,
        )
        .bind(message_id)
        .fetch_optional(pool)
        .await?;

        if let Some((state, processed_provider_events)) = delivery {
            if state == "delivered" && processed_provider_events > 0 {
                return Ok((state, processed_provider_events));
            }
            if is_terminal_failure(&state) {
                anyhow::bail!("synthetic email reached terminal state {state}");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(DELIVERY_POLL_SECONDS)).await;
    }
    anyhow::bail!("synthetic email was not delivered through a signed provider event in time")
}

fn is_terminal_failure(state: &str) -> bool {
    matches!(
        state,
        "hard_bounced" | "complained" | "suppressed" | "expired" | "dropped" | "failed"
    )
}

async fn submit_after_cold_start(
    config: &EmailClientConfig,
    command: &EmailCommand,
) -> Result<(EmailReceipt, u8), EmailClientError> {
    for attempt in 1..=MAX_COLD_START_ATTEMPTS {
        let result = match EmailClient::connect(config.clone()).await {
            Ok(client) => client.send(command.clone()).await,
            Err(error) => Err(error),
        };

        match result {
            Ok(receipt) => return Ok((receipt, attempt)),
            Err(EmailClientError::Unavailable) if attempt < MAX_COLD_START_ATTEMPTS => {
                tokio::time::sleep(std::time::Duration::from_secs(
                    COLD_START_RETRY_SECONDS[usize::from(attempt - 1)],
                ))
                .await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded retry loop always returns")
}

fn command(
    recipient: &str,
    run_id: &str,
    now: chrono::DateTime<Utc>,
) -> anyhow::Result<EmailCommand> {
    let idempotency_key = EmailIdempotencyKey::new(format!("readiness:{run_id}"))?;
    let deliver_before = now + Duration::minutes(30);
    let command = EmailCommand {
        context: EmailRequestContext {
            request_id: format!("readiness-{run_id}"),
            correlation_id: format!("readiness-{run_id}"),
            actor_principal_id: "operator:email-readiness".to_string(),
        },
        producer: "readiness".to_string(),
        idempotency_key,
        recipient: EmailRecipient {
            email: recipient.to_string(),
            name: Some("nvbes operator".to_string()),
        },
        category: EmailCategory::Operational,
        template: EmailTemplate::OperationalReadinessV1 {
            check_id: run_id.to_string(),
            environment: "production".to_string(),
        },
        deliver_before,
    };
    command.validate(now)?;
    Ok(command)
}

fn required_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

#[cfg(test)]
#[path = "email.worker.synthetic_smoke.tests.rs"]
mod tests;
