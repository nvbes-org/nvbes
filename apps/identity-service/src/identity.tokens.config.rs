use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct TokenConfig {
    pub issuer: String,
    pub key_id: String,
    pub private_key_pem: String,
    pub public_key_pem: String,
    pub allowed_audiences: BTreeSet<String>,
}

impl TokenConfig {
    pub fn from_env(environment: &str) -> anyhow::Result<Self> {
        Self::from_values(
            environment,
            required("NVBES_IDENTITY_TOKEN_ISSUER")?,
            required("NVBES_IDENTITY_TOKEN_KEY_ID")?,
            required("NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM")?,
            required("NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM")?,
            required("NVBES_IDENTITY_TOKEN_AUDIENCES")?,
        )
    }

    pub fn from_values(
        environment: &str,
        issuer: String,
        key_id: String,
        private_key_pem: String,
        public_key_pem: String,
        audiences: String,
    ) -> anyhow::Result<Self> {
        let issuer = issuer.trim_end_matches('/').to_string();
        let parsed = reqwest::Url::parse(&issuer)?;
        if parsed.host_str().is_none()
            || (!matches!(environment, "development" | "test") && parsed.scheme() != "https")
        {
            anyhow::bail!("Identity token issuer must be an HTTPS origin");
        }
        validate_identifier("key ID", &key_id)?;
        let allowed_audiences = audiences
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        if allowed_audiences.is_empty() {
            anyhow::bail!("Identity token audiences must not be empty");
        }
        for audience in &allowed_audiences {
            validate_identifier("audience", audience)?;
        }
        Ok(Self {
            issuer,
            key_id,
            private_key_pem,
            public_key_pem,
            allowed_audiences,
        })
    }
}

fn required(name: &str) -> anyhow::Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required"))
}

fn validate_identifier(label: &str, value: &str) -> anyhow::Result<()> {
    let valid = (3..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'));
    if !valid {
        anyhow::bail!("Identity token {label} is invalid");
    }
    Ok(())
}

#[cfg(test)]
#[path = "identity.tokens.config.tests.rs"]
mod tests;
