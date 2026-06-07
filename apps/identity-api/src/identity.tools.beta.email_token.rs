use anyhow::{Context, bail};

pub async fn extract_latest_email_token(
    pool: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    email: &str,
    business_type: &str,
) -> anyhow::Result<String> {
    let marker = email_token_marker(business_type)?;
    let jobs = nvbes_redis::worker_queue::find_matching_jobs(redis, "email.send", |job| {
        job.job_type == "email.send"
            && job
                .payload
                .get("to_email")
                .and_then(serde_json::Value::as_str)
                == Some(email)
            && job
                .payload
                .get("business_type")
                .and_then(serde_json::Value::as_str)
                == Some(business_type)
    })
    .await
    .context("Failed to scan Redis email jobs.")?;

    for job in jobs {
        let html_body = job
            .payload
            .get("html_body")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if let Some(token) = extract_token_from_body(html_body, marker) {
            if token_is_active(pool, redis, email, business_type, &token).await? {
                return Ok(token);
            }
        }

        let text_body = job
            .payload
            .get("text_body")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if let Some(token) = extract_token_from_body(text_body, marker) {
            if token_is_active(pool, redis, email, business_type, &token).await? {
                return Ok(token);
            }
        }
    }

    bail!("Unable to extract an active {business_type} token for the requested recipient.")
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
