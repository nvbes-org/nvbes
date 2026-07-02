use anyhow::{Context, bail};

#[path = "identity.tools.beta.account.rs"]
mod account;
#[path = "identity.tools.beta.email_token.rs"]
mod email_token;
#[path = "identity.tools.beta.seed.rs"]
mod seed;
#[path = "identity.tools.beta.workspace.rs"]
mod workspace;

const REQUIRED_PROVIDER_PRICE_PLAN_CODES: [&str; 3] = ["solo_pro", "team", "team_plus"];

pub enum BetaCliCommand {
    ExtractEmailToken {
        email: String,
        business_type: String,
    },
    PrepareBetaE2eAccount {
        email: String,
        password: String,
        workspace_name: String,
    },
    CheckProviderPriceMappings,
}

pub fn parse_cli_command(args: &[String]) -> anyhow::Result<Option<BetaCliCommand>> {
    if args
        .iter()
        .any(|arg| arg == "--check-provider-price-mappings" || arg == "--check-stripe-mappings")
    {
        return Ok(Some(BetaCliCommand::CheckProviderPriceMappings));
    }

    if args.iter().any(|arg| arg == "--prepare-beta-e2e-account") {
        let email = extract_arg_value(args, "--email")
            .context("Missing --email for --prepare-beta-e2e-account.")?;
        let password = extract_arg_value(args, "--password")
            .context("Missing --password for --prepare-beta-e2e-account.")?;
        let workspace_name = extract_arg_value(args, "--workspace-name")
            .context("Missing --workspace-name for --prepare-beta-e2e-account.")?;

        return Ok(Some(BetaCliCommand::PrepareBetaE2eAccount {
            email,
            password,
            workspace_name,
        }));
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
            let token =
                email_token::extract_latest_email_token(&pool, &redis, &email, &business_type)
                    .await?;
            println!("{token}");
        }
        BetaCliCommand::PrepareBetaE2eAccount {
            email,
            password,
            workspace_name,
        } => {
            seed::prepare_beta_e2e_account(
                &pool,
                &redis,
                seed::PrepareBetaE2eAccountInput {
                    email,
                    password,
                    workspace_name,
                },
            )
            .await?;
        }
        BetaCliCommand::CheckProviderPriceMappings => {
            check_active_provider_price_mappings(&pool).await?;
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

async fn check_active_provider_price_mappings(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    for plan_code in REQUIRED_PROVIDER_PRICE_PLAN_CODES {
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

        let mapping: Option<(String, String, String)> = sqlx::query_as(
            r#"
            SELECT provider::text, provider_product_id, provider_price_id
            FROM billing_provider_price_mappings
            WHERE legacy_plan_id = $1
              AND status = 'active'
            ORDER BY updated_at DESC, created_at DESC
            LIMIT 1
            "#,
        )
        .bind(plan_id)
        .fetch_optional(pool)
        .await
        .context("Failed to read provider price mappings.")?;

        let Some((provider, provider_product_id, provider_price_id)) = mapping else {
            bail!("Missing active provider price mapping for plan {plan_code}.");
        };

        if provider == "stripe"
            && (!provider_product_id.starts_with("prod_")
                || !provider_price_id.starts_with("price_"))
        {
            bail!(
                "Provider price mapping for plan {plan_code} does not look like a real Stripe test mapping."
            );
        }

        println!(
            "{plan_code}: provider={provider} product={provider_product_id} price={provider_price_id}"
        );
    }

    Ok(())
}
