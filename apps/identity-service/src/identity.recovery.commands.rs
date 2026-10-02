use crate::{config, database, mfa_crypto::MfaCrypto, recovery_delivery, required_secret};

/// Explicit operator commands. No scheduler, public signup or autonomous email sends.
pub async fn run(args: &[String]) -> anyhow::Result<bool> {
    let [action] = args else {
        return Ok(false);
    };
    if !matches!(
        action.as_str(),
        "request-password-recovery" | "dispatch-password-recovery"
    ) {
        return Ok(false);
    }
    let runtime = config::mfa_runtime_config_from_env()?;
    let crypto = MfaCrypto::with_rotation(
        runtime.key_version,
        runtime.encryption_key,
        runtime
            .previous_key_version
            .zip(runtime.previous_encryption_key),
    )?;
    let pool = database::connect(&runtime.database_url, 2).await?;
    if action == "request-password-recovery" {
        let email = required_secret("NVBES_IDENTITY_RECOVERY_EMAIL")?;
        let base_url = required_secret("NVBES_IDENTITY_RECOVERY_BASE_URL")?;
        recovery_delivery::enqueue(&pool, &crypto, &base_url, &email).await?;
        println!("{{\"queued\":true}}");
    } else {
        let email_config =
            nvbes_email::EmailClientConfig::from_env(&config::environment_from_env())?;
        let client = nvbes_email::EmailClient::connect(email_config).await?;
        let result = recovery_delivery::run_batch(&pool, &crypto, &client).await?;
        println!("{}", serde_json::to_string(&result)?);
    }
    pool.close().await;
    Ok(true)
}
