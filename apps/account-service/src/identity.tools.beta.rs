use anyhow::{Context, bail};

#[path = "identity.tools.beta.account.rs"]
mod account;
#[path = "identity.tools.beta.email_token.rs"]
mod email_token;
#[path = "identity.tools.beta.seed.rs"]
mod seed;
#[path = "identity.tools.beta.workspace.rs"]
mod workspace;

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
}

pub fn parse_cli_command(args: &[String]) -> anyhow::Result<Option<BetaCliCommand>> {
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
