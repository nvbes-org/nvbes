use anyhow::{Context, bail};

const REQUIRED_STRIPE_PLAN_CODES: [&str; 3] = ["solo_pro", "team", "team_plus"];

pub enum BetaCliCommand {
    ExtractEmailToken {
        email: String,
        business_type: String,
    },
    CheckStripeMappings,
}

pub fn parse_cli_command(args: &[String]) -> anyhow::Result<Option<BetaCliCommand>> {
    if args.iter().any(|arg| arg == "--check-stripe-mappings") {
        return Ok(Some(BetaCliCommand::CheckStripeMappings));
    }

    if !args.iter().any(|arg| arg == "--extract-email-token") {
        return Ok(None);
    }

    let email =
        extract_arg_value(args, "--email").context("Missing --email for --extract-email-token.")?;
    let business_type = extract_arg_value(args, "--business-type")
        .context("Missing --business-type for --extract-email-token.")?;

    Ok(Some(BetaCliCommand::ExtractEmailToken {
        email,
        business_type,
    }))
}

pub async fn run_cli_command(command: BetaCliCommand) -> anyhow::Result<()> {
    ensure_non_production()?;

    let database_url = std::env::var("NVBES_DATABASE_URL")
        .context("NVBES_DATABASE_URL is required for beta CLI commands.")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .context("Failed to connect to the database.")?;
    let redis_config = nvbes_redis::RedisConfig::from_env();
    let redis = nvbes_redis::connection::create_pool(&redis_config)
        .await
        .context("Failed to connect to Redis.")?;
    nvbes_redis::connection::health_check(&redis)
        .await
        .context("Redis health check failed.")?;

    match command {
        BetaCliCommand::ExtractEmailToken {
            email,
            business_type,
        } => {
            let token = extract_latest_email_token(&redis, &email, &business_type).await?;
            println!("{token}");
        }
        BetaCliCommand::CheckStripeMappings => {
            check_active_stripe_mappings(&pool).await?;
        }
    }

    Ok(())
}

fn ensure_non_production() -> anyhow::Result<()> {
    if matches!(std::env::var("NVBES_ENV").as_deref(), Ok("production")) {
        bail!("This helper is disabled in production.");
    }

    Ok(())
}

fn extract_arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

async fn extract_latest_email_token(
    redis: &nvbes_redis::RedisPool,
    email: &str,
    business_type: &str,
) -> anyhow::Result<String> {
    let marker = email_token_marker(business_type)?;
    let job = nvbes_redis::worker_queue::find_latest_job(redis, "email.send", |job| {
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
    .context("Failed to scan Redis email jobs.")?
    .context("No queued email job found for the requested recipient.")?;

    let html_body = job
        .payload
        .get("html_body")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let text_body = job
        .payload
        .get("text_body")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    if let Some(token) = extract_token_from_body(html_body, marker) {
        return Ok(token);
    }

    if let Some(token) = extract_token_from_body(text_body, marker) {
        return Ok(token);
    }

    bail!("Unable to extract the {business_type} token from the latest email payload.")
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

async fn check_active_stripe_mappings(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    for plan_code in REQUIRED_STRIPE_PLAN_CODES {
        let plan_id: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
            FROM plans
            WHERE code = $1
            "#,
        )
        .bind(plan_code)
        .fetch_optional(pool)
        .await
        .context("Failed to read plan records.")?;

        let Some(plan_id) = plan_id else {
            bail!("Plan {plan_code} is missing.");
        };

        let mapping: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT stripe_product_id, stripe_price_id
            FROM stripe_price_mappings
            WHERE plan_id = $1
              AND meter = 'subscription'
              AND status = 'active'
              AND valid_from <= NOW()
              AND (valid_until IS NULL OR valid_until > NOW())
            ORDER BY valid_from DESC
            LIMIT 1
            "#,
        )
        .bind(plan_id)
        .fetch_optional(pool)
        .await
        .context("Failed to read Stripe price mappings.")?;

        let Some((stripe_product_id, stripe_price_id)) = mapping else {
            bail!("Missing active Stripe mapping for plan {plan_code}.");
        };

        if !stripe_product_id.starts_with("prod_") || !stripe_price_id.starts_with("price_") {
            bail!(
                "Stripe mapping for plan {plan_code} does not look like a real Stripe test mapping."
            );
        }

        println!("{plan_code}: product={stripe_product_id} price={stripe_price_id}");
    }

    Ok(())
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
