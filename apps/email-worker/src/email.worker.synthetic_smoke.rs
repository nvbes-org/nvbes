use chrono::{Duration, Utc};
use nvbes_email::{
    EmailCategory, EmailClient, EmailClientConfig, EmailClientError, EmailCommand,
    EmailIdempotencyKey, EmailReceipt, EmailRecipient, EmailRequestContext, EmailTemplate,
};
use serde::Serialize;

const RECIPIENT_ENV: &str = "NVBES_EMAIL_SYNTHETIC_RECIPIENT";
const RUN_ID_ENV: &str = "NVBES_EMAIL_SYNTHETIC_RUN_ID";
const MAX_COLD_START_ATTEMPTS: u8 = 5;
const COLD_START_RETRY_SECONDS: [u64; 4] = [1, 2, 4, 8];

#[derive(Serialize)]
struct SyntheticSmokeReceipt {
    message_id: String,
    accepted_at: chrono::DateTime<Utc>,
    deliver_before: chrono::DateTime<Utc>,
    duplicate: bool,
    attempts: u8,
}

pub async fn run() -> anyhow::Result<()> {
    let recipient = required_env(RECIPIENT_ENV)?;
    let run_id = required_env(RUN_ID_ENV)?;
    let command = command(&recipient, &run_id, Utc::now())?;
    let client = EmailClient::connect(EmailClientConfig::from_env("production")?).await?;
    let (receipt, attempts) = submit_after_cold_start(&client, &command).await?;

    println!(
        "{}",
        serde_json::to_string(&SyntheticSmokeReceipt {
            message_id: receipt.message_id,
            accepted_at: receipt.accepted_at,
            deliver_before: receipt.deliver_before,
            duplicate: receipt.duplicate,
            attempts,
        })?
    );
    Ok(())
}

async fn submit_after_cold_start(
    client: &EmailClient,
    command: &EmailCommand,
) -> Result<(EmailReceipt, u8), EmailClientError> {
    for attempt in 1..=MAX_COLD_START_ATTEMPTS {
        match client.send(command.clone()).await {
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
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn builds_an_explicit_bounded_operational_message() {
        let now = Utc.with_ymd_and_hms(2026, 8, 25, 20, 0, 0).unwrap();
        let command = command("operator@example.com", "deploy-123", now).unwrap();

        assert_eq!(command.category, EmailCategory::Operational);
        assert_eq!(command.deliver_before, now + Duration::minutes(30));
        let rendered = command.template.render();
        assert!(rendered.subject.contains("no action required"));
        assert!(rendered.text_body.contains("deploy-123"));
        assert!(!rendered.text_body.contains("operator@example.com"));
    }

    #[test]
    fn rejects_an_unbounded_or_unsafe_run_identifier() {
        let now = Utc.with_ymd_and_hms(2026, 8, 25, 20, 0, 0).unwrap();

        assert!(command("operator@example.com", "contains spaces", now).is_err());
    }

    #[test]
    fn cold_start_retry_schedule_is_bounded() {
        assert_eq!(MAX_COLD_START_ATTEMPTS, 5);
        assert_eq!(COLD_START_RETRY_SECONDS, [1, 2, 4, 8]);
        assert_eq!(COLD_START_RETRY_SECONDS.iter().sum::<u64>(), 15);
    }
}
