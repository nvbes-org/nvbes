use super::{DispatchMode, DispatchQueueConfig, env_or, optional, required};

pub fn from_environment(environment: &str) -> anyhow::Result<DispatchMode> {
    if matches!(environment, "development" | "test") && optional("NVBES_EMAIL_QUEUE_URL").is_none()
    {
        return Ok(DispatchMode::InMemory);
    }

    let region = env_or("NVBES_EMAIL_QUEUE_REGION", "fr-par");
    let endpoint = env_or(
        "NVBES_EMAIL_QUEUE_ENDPOINT",
        &format!("https://sqs.mnq.{region}.scaleway.com"),
    );
    if !endpoint.starts_with("https://") {
        anyhow::bail!("NVBES_EMAIL_QUEUE_ENDPOINT must use HTTPS");
    }
    Ok(DispatchMode::Scaleway(DispatchQueueConfig {
        endpoint,
        queue_url: required("NVBES_EMAIL_QUEUE_URL")?,
        access_key: required("NVBES_EMAIL_QUEUE_ACCESS_KEY")?,
        secret_key: required("NVBES_EMAIL_QUEUE_SECRET_KEY")?,
        region,
    }))
}
