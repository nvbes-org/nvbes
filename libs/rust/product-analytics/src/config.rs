#[derive(Debug, Clone)]
pub struct ProductAnalyticsConfig {
    pub enabled: bool,
    pub host: String,
    pub project_token: Option<String>,
    pub analytics_id_salt: Option<String>,
}

impl ProductAnalyticsConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: env_bool("NVBES_POSTHOG_ENABLED", false),
            host: optional_env("NVBES_POSTHOG_HOST")
                .unwrap_or_else(|| "https://eu.i.posthog.com".to_string()),
            project_token: optional_env("NVBES_POSTHOG_PROJECT_TOKEN"),
            analytics_id_salt: optional_env("NVBES_ANALYTICS_ID_SALT"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProductAnalyticsError {
    #[error("NVBES_POSTHOG_HOST must be a valid HTTP(S) URL")]
    InvalidHost,
    #[error("NVBES_POSTHOG_PROJECT_TOKEN is required when PostHog analytics is enabled")]
    MissingProjectToken,
    #[error("NVBES_ANALYTICS_ID_SALT is required when PostHog analytics is enabled")]
    MissingAnalyticsSalt,
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| v == "true" || v == "1")
        .unwrap_or(default)
}
