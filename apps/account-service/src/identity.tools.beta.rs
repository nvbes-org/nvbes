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
        workspace_name: String,
    },
}

pub fn parse_cli_command(args: &[String]) -> anyhow::Result<Option<BetaCliCommand>> {
    if args.iter().any(|arg| arg == "--prepare-beta-e2e-account") {
        let email = extract_arg_value(args, "--email")
            .context("Missing --email for --prepare-beta-e2e-account.")?;
        let workspace_name = extract_arg_value(args, "--workspace-name")
            .context("Missing --workspace-name for --prepare-beta-e2e-account.")?;

        return Ok(Some(BetaCliCommand::PrepareBetaE2eAccount {
            email,
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
    let database_url = std::env::var("NVBES_DATABASE_URL")
        .context("NVBES_DATABASE_URL is required for beta CLI commands.")?;
    ensure_disposable_test_database(&database_url)?;
    ensure_loopback_redis()?;
    match &command {
        BetaCliCommand::ExtractEmailToken { .. } => ensure_test_capture_directory()?,
        BetaCliCommand::PrepareBetaE2eAccount { .. } => ensure_loopback_cloud()?,
    }
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
            workspace_name,
        } => {
            let password = std::env::var("NVBES_BETA_SEED_PASSWORD")
                .context("NVBES_BETA_SEED_PASSWORD is required for beta account seeding.")?;
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

fn ensure_disposable_test_database(database_url: &str) -> anyhow::Result<()> {
    validate_disposable_test_database(
        database_url,
        std::env::var("NVBES_ENV").ok().as_deref(),
        std::env::var("NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE")
            .ok()
            .as_deref(),
    )
}

fn validate_disposable_test_database(
    database_url: &str,
    environment: Option<&str>,
    destructive_opt_in: Option<&str>,
) -> anyhow::Result<()> {
    nvbes_account_service::test_support::validate_test_database_url(
        database_url,
        environment,
        destructive_opt_in,
    )
    .map_err(anyhow::Error::msg)
}

fn ensure_loopback_redis() -> anyhow::Result<()> {
    let redis_url = std::env::var("NVBES_REDIS_URL")
        .context("NVBES_REDIS_URL is required for beta CLI commands.")?;
    let url = url::Url::parse(&redis_url).context("NVBES_REDIS_URL must be a valid URL.")?;
    ensure_loopback_host(&url, "NVBES_REDIS_URL")
}

fn ensure_loopback_cloud() -> anyhow::Result<()> {
    let endpoint = std::env::var("NVBES_CLOUD_GRPC_ENDPOINT")
        .context("NVBES_CLOUD_GRPC_ENDPOINT is required for beta account seeding.")?;
    let url =
        url::Url::parse(&endpoint).context("NVBES_CLOUD_GRPC_ENDPOINT must be a valid URL.")?;
    ensure_loopback_host(&url, "NVBES_CLOUD_GRPC_ENDPOINT")
}

fn ensure_test_capture_directory() -> anyhow::Result<()> {
    let directory = std::env::var("NVBES_EMAIL_TEST_CAPTURE_DIR")
        .context("NVBES_EMAIL_TEST_CAPTURE_DIR is required for email token extraction.")?;
    let directory = std::path::Path::new(&directory);
    if !directory.is_absolute() {
        bail!("NVBES_EMAIL_TEST_CAPTURE_DIR must be absolute.");
    }
    let metadata = std::fs::symlink_metadata(directory)
        .context("NVBES_EMAIL_TEST_CAPTURE_DIR must already exist.")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("NVBES_EMAIL_TEST_CAPTURE_DIR must be a real directory.");
    }
    Ok(())
}

fn ensure_loopback_host(url: &url::Url, label: &str) -> anyhow::Result<()> {
    let host = url
        .host_str()
        .with_context(|| format!("{label} must contain a host."))?;
    if !matches!(host, "localhost" | "127.0.0.1" | "::1") {
        bail!("{label} must use loopback infrastructure; refusing host {host:?}.");
    }
    Ok(())
}

fn extract_arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

#[cfg(test)]
mod tests {
    use super::validate_disposable_test_database;

    #[test]
    fn beta_cli_accepts_only_loopback_test_databases() {
        assert!(
            validate_disposable_test_database(
                "postgres://postgres:postgres@127.0.0.1:5432/nvbes_account_test",
                Some("test"),
                Some("account-quality-v1"),
            )
            .is_ok()
        );
        assert!(
            validate_disposable_test_database(
                "postgres://postgres:postgres@database.internal:5432/nvbes_account_test",
                Some("test"),
                Some("account-quality-v1"),
            )
            .is_err()
        );
        assert!(
            validate_disposable_test_database(
                "postgres://postgres:postgres@127.0.0.1:5432/nvbes",
                Some("test"),
                Some("account-quality-v1"),
            )
            .is_err()
        );
        assert!(
            validate_disposable_test_database(
                "postgres://postgres:postgres@127.0.0.1:5432/latest_production",
                Some("test"),
                Some("account-quality-v1"),
            )
            .is_err()
        );
        assert!(
            validate_disposable_test_database(
                "postgres://postgres:postgres@127.0.0.1:5432/nvbes_account_test",
                Some("test"),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn beta_cli_is_disabled_in_production() {
        assert!(
            validate_disposable_test_database(
                "postgres://postgres:postgres@127.0.0.1:5432/nvbes_account_test",
                Some("production"),
                Some("account-quality-v1"),
            )
            .is_err()
        );
    }
}
