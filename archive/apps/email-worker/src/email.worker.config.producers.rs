use std::collections::{HashMap, HashSet};

const DEVELOPMENT_PRODUCER_TOKENS: &str = "identity-service=development-email-internal-token-32,billing-service=development-email-internal-token-32";
const MINIMUM_PRODUCER_TOKEN_LENGTH: usize = 32;

pub fn from_environment(
    configured: Option<String>,
    environment: &str,
) -> anyhow::Result<HashMap<String, String>> {
    let value = configured.unwrap_or_else(|| {
        if matches!(environment, "development" | "test") {
            DEVELOPMENT_PRODUCER_TOKENS.to_string()
        } else {
            String::new()
        }
    });
    parse(&value, environment)
}

pub fn parse(value: &str, environment: &str) -> anyhow::Result<HashMap<String, String>> {
    let mut producers = HashMap::new();
    for entry in value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let (producer, token) = entry.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("NVBES_EMAIL_PRODUCER_TOKENS entries must use producer=token")
        })?;
        let producer = producer.trim();
        let token = token.trim();
        if producer.is_empty()
            || !producer
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            anyhow::bail!("NVBES_EMAIL_PRODUCER_TOKENS contains an invalid producer");
        }
        if token.len() < MINIMUM_PRODUCER_TOKEN_LENGTH || !token.is_ascii() {
            anyhow::bail!(
                "NVBES_EMAIL_PRODUCER_TOKENS tokens must contain at least {MINIMUM_PRODUCER_TOKEN_LENGTH} ASCII characters"
            );
        }
        if producers
            .insert(producer.to_string(), token.to_string())
            .is_some()
        {
            anyhow::bail!("NVBES_EMAIL_PRODUCER_TOKENS contains a duplicate producer");
        }
    }
    if producers.is_empty() {
        anyhow::bail!("NVBES_EMAIL_PRODUCER_TOKENS must not be empty");
    }
    if !matches!(environment, "development" | "test") {
        let mut unique_tokens = HashSet::new();
        if !producers
            .values()
            .all(|token| unique_tokens.insert(token.as_str()))
        {
            anyhow::bail!("NVBES_EMAIL_PRODUCER_TOKENS must use one token per producer");
        }
    }
    Ok(producers)
}
