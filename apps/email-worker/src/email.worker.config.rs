use std::{collections::HashSet, net::SocketAddr, path::PathBuf};

use base64::{Engine, engine::general_purpose::STANDARD};

const INTERNAL_TOKEN_ENV: &str = "NVBES_EMAIL_GRPC_AUTH_TOKEN";
const DEVELOPMENT_TOKEN: &str = "development-email-internal-token-32";
const DEVELOPMENT_DATA_KEY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
const DEVELOPMENT_HMAC_KEY: &str = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=";

#[derive(Debug, Clone)]
pub struct ScalewayConfig {
    pub secret_key: String,
    pub project_id: String,
    pub region: String,
}

#[derive(Debug, Clone)]
pub enum ProviderConfig {
    Mock,
    Smtp(nvbes_email::SmtpEmailConfig),
    TestCapture(PathBuf),
    Scaleway(ScalewayConfig),
}

#[derive(Debug, Clone)]
pub struct WebhookTrustConfig {
    pub topic_arn: String,
    pub ca_bundle_pem: Vec<u8>,
    pub signing_certificate_host: String,
    pub confirmation_host: String,
}

#[derive(Debug, Clone)]
pub struct EmailWorkerConfig {
    pub environment: String,
    pub database_url: String,
    pub http_bind_addr: SocketAddr,
    pub grpc_bind_addr: SocketAddr,
    pub internal_token: String,
    pub allowed_producers: HashSet<String>,
    pub data_encryption_key: [u8; 32],
    pub recipient_hmac_key: [u8; 32],
    pub from_email: String,
    pub from_name: String,
    pub reply_to: Option<String>,
    pub message_id_domain: String,
    pub provider: ProviderConfig,
    pub webhook: Option<WebhookTrustConfig>,
}

impl EmailWorkerConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let environment = env_or("NVBES_ENVIRONMENT", "development");
        let database_url = required("NVBES_EMAIL_DATABASE_URL")?;
        let http_bind_addr = env_or("NVBES_EMAIL_HTTP_BIND_ADDR", "127.0.0.1:3040")
            .parse()
            .map_err(|error| anyhow::anyhow!("NVBES_EMAIL_HTTP_BIND_ADDR is invalid: {error}"))?;
        let grpc_bind_addr = env_or("NVBES_EMAIL_GRPC_BIND_ADDR", "127.0.0.1:3041")
            .parse()
            .map_err(|error| anyhow::anyhow!("NVBES_EMAIL_GRPC_BIND_ADDR is invalid: {error}"))?;
        let internal_token = nvbes_core::http::internal_service::load_token(
            INTERNAL_TOKEN_ENV,
            &environment,
            DEVELOPMENT_TOKEN,
        )
        .map_err(anyhow::Error::msg)?;
        let allowed_producers = csv_set(&env_or(
            "NVBES_EMAIL_ALLOWED_PRODUCERS",
            "identity-service,identity-worker,billing-worker,enterprise-service",
        ))?;
        let data_encryption_key = key(
            "NVBES_EMAIL_DATA_ENCRYPTION_KEY",
            development_value(&environment, DEVELOPMENT_DATA_KEY),
        )?;
        let recipient_hmac_key = key(
            "NVBES_EMAIL_RECIPIENT_HMAC_KEY",
            development_value(&environment, DEVELOPMENT_HMAC_KEY),
        )?;
        let from_email = required("NVBES_EMAIL_FROM_EMAIL")?;
        from_email
            .parse::<lettre::Address>()
            .map_err(|_| anyhow::anyhow!("NVBES_EMAIL_FROM_EMAIL is invalid"))?;
        let from_name = env_or("NVBES_EMAIL_FROM_NAME", "nvbes");
        let reply_to = optional("NVBES_EMAIL_REPLY_TO");
        if let Some(reply_to) = reply_to.as_deref() {
            reply_to
                .parse::<lettre::Address>()
                .map_err(|_| anyhow::anyhow!("NVBES_EMAIL_REPLY_TO is invalid"))?;
        }
        let message_id_domain = optional("NVBES_EMAIL_MESSAGE_ID_DOMAIN")
            .or_else(|| {
                from_email
                    .split_once('@')
                    .map(|(_, domain)| domain.to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("email message ID domain is missing"))?;
        let provider = provider(&environment)?;
        let webhook = webhook(&environment)?;

        Ok(Self {
            environment,
            database_url,
            http_bind_addr,
            grpc_bind_addr,
            internal_token,
            allowed_producers,
            data_encryption_key,
            recipient_hmac_key,
            from_email,
            from_name,
            reply_to,
            message_id_domain,
            provider,
            webhook,
        })
    }
}

fn webhook(environment: &str) -> anyhow::Result<Option<WebhookTrustConfig>> {
    let topic_arn = optional("NVBES_EMAIL_SNS_TOPIC_ARN");
    let ca_path = optional("NVBES_EMAIL_SNS_CA_BUNDLE_PATH");
    if topic_arn.is_none() && ca_path.is_none() && matches!(environment, "development" | "test") {
        return Ok(None);
    }
    let topic_arn = topic_arn.ok_or_else(|| {
        anyhow::anyhow!("NVBES_EMAIL_SNS_TOPIC_ARN is required outside development")
    })?;
    let ca_path = ca_path.ok_or_else(|| {
        anyhow::anyhow!("NVBES_EMAIL_SNS_CA_BUNDLE_PATH is required outside development")
    })?;
    let ca_bundle_pem = std::fs::read(&ca_path)
        .map_err(|error| anyhow::anyhow!("could not read SNS CA bundle {ca_path}: {error}"))?;
    if ca_bundle_pem.is_empty() {
        anyhow::bail!("NVBES_EMAIL_SNS_CA_BUNDLE_PATH is empty");
    }
    Ok(Some(WebhookTrustConfig {
        topic_arn,
        ca_bundle_pem,
        signing_certificate_host: env_or(
            "NVBES_EMAIL_SNS_CERTIFICATE_HOST",
            "messaging.s3.fr-par.scw.cloud",
        ),
        confirmation_host: env_or(
            "NVBES_EMAIL_SNS_CONFIRMATION_HOST",
            "sns.mnq.fr-par.scaleway.com",
        ),
    }))
}

fn provider(environment: &str) -> anyhow::Result<ProviderConfig> {
    match env_or(
        "NVBES_EMAIL_PROVIDER",
        if environment == "development" {
            "smtp"
        } else {
            "scaleway"
        },
    )
    .as_str()
    {
        "mock" if matches!(environment, "development" | "test") => Ok(ProviderConfig::Mock),
        "mock" => anyhow::bail!("mock email provider is forbidden outside development and test"),
        "smtp" => Ok(ProviderConfig::Smtp(local_smtp(
            environment,
            env_or("NVBES_SMTP_HOST", "127.0.0.1"),
            env_or("NVBES_SMTP_PORT", "11025")
                .parse()
                .map_err(|error| anyhow::anyhow!("NVBES_SMTP_PORT is invalid: {error}"))?,
            optional("NVBES_SMTP_USERNAME"),
            optional("NVBES_SMTP_PASSWORD"),
            env_or("NVBES_SMTP_STARTTLS", "false")
                .parse()
                .map_err(|error| anyhow::anyhow!("NVBES_SMTP_STARTTLS is invalid: {error}"))?,
        )?)),
        "test-capture" if matches!(environment, "development" | "test") => {
            let directory = PathBuf::from(required("NVBES_EMAIL_TEST_CAPTURE_DIR")?);
            if !directory.is_absolute() {
                anyhow::bail!("NVBES_EMAIL_TEST_CAPTURE_DIR must be absolute");
            }
            Ok(ProviderConfig::TestCapture(directory))
        }
        "test-capture" => {
            anyhow::bail!("test-capture email provider is forbidden outside development and test")
        }
        "scaleway" => Ok(ProviderConfig::Scaleway(ScalewayConfig {
            secret_key: required("NVBES_SCALEWAY_EMAIL_SECRET_KEY")?,
            project_id: required("NVBES_SCALEWAY_EMAIL_PROJECT_ID")?,
            region: env_or("NVBES_SCALEWAY_EMAIL_REGION", "fr-par"),
        })),
        value => anyhow::bail!("unsupported NVBES_EMAIL_PROVIDER={value}"),
    }
}

fn local_smtp(
    environment: &str,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    starttls: bool,
) -> anyhow::Result<nvbes_email::SmtpEmailConfig> {
    if !matches!(environment, "development" | "test") {
        anyhow::bail!("smtp email provider is forbidden outside development and test");
    }
    if host.trim().is_empty() {
        anyhow::bail!("NVBES_SMTP_HOST must not be empty");
    }
    if username.is_some() != password.is_some() {
        anyhow::bail!("NVBES_SMTP_USERNAME and NVBES_SMTP_PASSWORD must be configured together");
    }
    Ok(nvbes_email::SmtpEmailConfig {
        host,
        port,
        username,
        password,
        starttls,
    })
}

fn key(variable: &str, fallback: Option<&str>) -> anyhow::Result<[u8; 32]> {
    let encoded = optional(variable)
        .or_else(|| fallback.map(str::to_string))
        .ok_or_else(|| anyhow::anyhow!("{variable} is required"))?;
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| anyhow::anyhow!("{variable} must be base64"))?;
    decoded
        .try_into()
        .map_err(|_| anyhow::anyhow!("{variable} must decode to exactly 32 bytes"))
}

fn csv_set(value: &str) -> anyhow::Result<HashSet<String>> {
    let values: HashSet<_> = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect();
    if values.is_empty() {
        anyhow::bail!("NVBES_EMAIL_ALLOWED_PRODUCERS must not be empty");
    }
    Ok(values)
}

fn development_value<'a>(environment: &str, value: &'a str) -> Option<&'a str> {
    matches!(environment, "development" | "test").then_some(value)
}

fn required(variable: &str) -> anyhow::Result<String> {
    optional(variable).ok_or_else(|| anyhow::anyhow!("{variable} is required"))
}

fn optional(variable: &str) -> Option<String> {
    std::env::var(variable)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_or(variable: &str, fallback: &str) -> String {
    optional(variable).unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
#[path = "email.worker.config.tests.rs"]
mod tests;
