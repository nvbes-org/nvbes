use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, bail};
use serde::Deserialize;
use tokio::time::{Instant, sleep};

const DELIVERY_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
const DELIVERY_POLL_INTERVAL: Duration = Duration::from_millis(250);
const MAX_CAPTURE_BYTES: u64 = 1_048_576;

pub async fn extract_latest_email_token(
    pool: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    email: &str,
    business_type: &str,
) -> anyhow::Result<String> {
    let marker = email_token_marker(business_type)?;
    let capture_directory = test_capture_directory()?;
    let deadline = Instant::now() + DELIVERY_WAIT_TIMEOUT;

    loop {
        for (path, capture) in matching_captures(&capture_directory, email, business_type).await? {
            if !email_delivery_completed(pool, capture.job_id).await? {
                continue;
            }

            for body in [
                capture.html_body.as_deref().unwrap_or_default(),
                capture.text_body.as_deref().unwrap_or_default(),
            ] {
                if let Some(token) = extract_token_from_body(body, marker)
                    && token_is_active(pool, redis, email, business_type, &token).await?
                {
                    tokio::fs::remove_file(&path)
                        .await
                        .context("Failed to consume the test email capture.")?;
                    return Ok(token);
                }
            }
        }

        if Instant::now() >= deadline {
            break;
        }
        sleep(DELIVERY_POLL_INTERVAL).await;
    }

    bail!(
        "Timed out waiting for a completed {business_type} email delivery for the requested recipient."
    )
}

async fn matching_captures(
    directory: &Path,
    email: &str,
    business_type: &str,
) -> anyhow::Result<Vec<(PathBuf, TestEmailCapture)>> {
    let mut entries = tokio::fs::read_dir(directory)
        .await
        .context("Failed to read the test email capture directory.")?;
    let mut captures = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .context("Failed to enumerate test email captures.")?
    {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with("capture-") || !file_name.ends_with(".json") {
            continue;
        }
        let metadata = entry
            .metadata()
            .await
            .context("Failed to inspect a test email capture.")?;
        if !metadata.is_file() || metadata.len() > MAX_CAPTURE_BYTES {
            bail!("Test email capture is not a bounded regular file.");
        }
        let contents = tokio::fs::read(&path)
            .await
            .context("Failed to read a test email capture.")?;
        let capture: TestEmailCapture =
            serde_json::from_slice(&contents).context("Test email capture is invalid.")?;
        if capture.business_type == business_type
            && capture.to.iter().any(|recipient| recipient == email)
            && file_name == format!("capture-{}.json", capture.job_id)
        {
            captures.push((path, capture));
        }
    }
    Ok(captures)
}

async fn email_delivery_completed(pool: &sqlx::PgPool, job_id: uuid::Uuid) -> anyhow::Result<bool> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status::text FROM email_messages WHERE job_id = $1 LIMIT 1")
            .bind(job_id)
            .fetch_optional(pool)
            .await
            .context("Failed to read the durable email delivery ledger.")?;

    Ok(matches!(status.as_deref(), Some("sent" | "delivered")))
}

fn test_capture_directory() -> anyhow::Result<PathBuf> {
    let directory = std::env::var("NVBES_EMAIL_TEST_CAPTURE_DIR")
        .context("NVBES_EMAIL_TEST_CAPTURE_DIR is required for email token extraction.")?;
    let directory = PathBuf::from(directory);
    if !directory.is_absolute() {
        bail!("NVBES_EMAIL_TEST_CAPTURE_DIR must be absolute.");
    }
    Ok(directory)
}

#[derive(Deserialize)]
struct TestEmailCapture {
    job_id: uuid::Uuid,
    business_type: String,
    to: Vec<String>,
    text_body: Option<String>,
    html_body: Option<String>,
}

fn email_token_marker(business_type: &str) -> anyhow::Result<&'static str> {
    match business_type {
        "verification" => Ok("verify-result?token="),
        "password_reset" => Ok("reset-password?token="),
        "invitation" => Ok("join?token="),
        other => bail!("Unsupported business type: {other}"),
    }
}

fn extract_token_from_body(body: &str, marker: &str) -> Option<String> {
    let start = body.find(marker)? + marker.len();
    let token = body[start..]
        .chars()
        .take_while(|ch| !matches!(ch, '&' | '"' | '\'' | '<' | '>' | ' ' | '\n' | '\r' | '\t'))
        .collect::<String>();

    if token.is_empty() { None } else { Some(token) }
}

async fn token_is_active(
    pool: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    email: &str,
    business_type: &str,
    token: &str,
) -> anyhow::Result<bool> {
    let token_hash = nvbes_core::auth::token_hash(token);
    let principal_id = principal_id_by_email(pool, email).await?;

    match business_type {
        "password_reset" => {
            let Some(cached) =
                nvbes_redis::password_reset::get_password_reset_token(redis, &token_hash)
                    .await
                    .context("Failed to read password reset token cache.")?
            else {
                return Ok(false);
            };

            Ok(cached.principal_id == principal_id
                && cached.consumed_at.is_none()
                && cached.expires_at > chrono::Utc::now())
        }
        "verification" => {
            let Some(cached) =
                nvbes_redis::email_verification::get_email_verification_token(redis, &token_hash)
                    .await
                    .context("Failed to read email verification token cache.")?
            else {
                return Ok(false);
            };

            Ok(cached.principal_id == principal_id
                && cached.consumed_at.is_none()
                && cached.expires_at > chrono::Utc::now())
        }
        _ => Ok(true),
    }
}

async fn principal_id_by_email(pool: &sqlx::PgPool, email: &str) -> anyhow::Result<uuid::Uuid> {
    sqlx::query_scalar(
        r#"
        SELECT principal_id
        FROM users
        WHERE lower(email) = lower($1)
        LIMIT 1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .context("Failed to resolve token recipient principal.")?
    .context("Token recipient principal does not exist.")
}

#[cfg(test)]
mod tests {
    use super::extract_token_from_body;

    #[test]
    fn extracts_token_from_url_marker() {
        let body = "https://example.test/verify-result?token=abc123_xyz";
        assert_eq!(
            extract_token_from_body(body, "verify-result?token="),
            Some("abc123_xyz".to_string())
        );
    }

    #[test]
    fn stops_at_html_delimiters() {
        let body = r#"<a href="https://example.test/reset-password?token=abc123_xyz&foo=bar">"#;
        assert_eq!(
            extract_token_from_body(body, "reset-password?token="),
            Some("abc123_xyz".to_string())
        );
    }
}
