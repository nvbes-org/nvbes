use reqwest::Client;
use serde::Deserialize;

use super::scw_kms::KmsConfig;

#[derive(Debug, Clone)]
pub struct SecretResolver {
    http: Client,
    config: KmsConfig,
    prefix: String,
}

#[derive(Debug, Deserialize)]
struct SecretListResponse {
    secrets: Vec<SecretMeta>,
    #[serde(rename = "total_count")]
    _total_count: u64,
}

#[derive(Debug, Deserialize)]
struct SecretMeta {
    id: String,
    name: String,
    #[serde(rename = "status")]
    _status: String,
}

#[derive(Debug, Deserialize)]
struct SecretVersionAccess {
    data: String,
}

impl SecretResolver {
    pub fn new(config: &KmsConfig, environment: &str) -> Self {
        let prefix = format!("nvbes/{}/", environment);
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("valid HTTP client"),
            config: config.clone(),
            prefix,
        }
    }

    pub async fn resolve_secrets(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, String> {
        let all_secrets = self.list_all_secrets().await?;
        let mut resolved = std::collections::HashMap::new();

        for secret in all_secrets {
            let env_key = secret_name_to_env_key(&self.prefix, &secret.name);
            if env_key.is_empty() {
                continue;
            }
            match self.access_latest(&secret.id).await {
                Ok(value) => {
                    if !value.is_empty() {
                        resolved.insert(env_key, value);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to access secret {} ({}): {}",
                        secret.name,
                        secret.id,
                        e
                    );
                }
            }
        }

        Ok(resolved)
    }

    async fn list_all_secrets(&self) -> Result<Vec<SecretMeta>, String> {
        let url = format!(
            "{}/secret-manager/v1alpha1/regions/{}/secrets?project_id={}",
            self.config.api_base(),
            self.config.region,
            self.config.project_id
        );

        let resp = self
            .http
            .get(&url)
            .header("X-Auth-Token", &self.config.secret_key);
        let resp = crate::trace_context::with_fresh_trace_headers(resp)
            .send()
            .await
            .map_err(|e| format!("Secret Manager API error: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Secret Manager returned {status}: {body}"));
        }

        let list: SecretListResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse secret list: {e}"))?;

        Ok(list
            .secrets
            .into_iter()
            .filter(|s| s.name.starts_with(&self.prefix))
            .collect())
    }

    async fn access_latest(&self, secret_id: &str) -> Result<String, String> {
        let url = format!(
            "{}/secret-manager/v1alpha1/regions/{}/secrets/{}/versions/latest/access",
            self.config.api_base(),
            self.config.region,
            secret_id
        );

        let resp = self
            .http
            .get(&url)
            .header("X-Auth-Token", &self.config.secret_key);
        let resp = crate::trace_context::with_fresh_trace_headers(resp)
            .send()
            .await
            .map_err(|e| format!("Secret access error: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Secret access returned {status}: {body}"));
        }

        let access: SecretVersionAccess = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse secret: {e}"))?;

        let decoded = base64_decoded(&access.data);
        Ok(decoded)
    }
}

fn secret_name_to_env_key(prefix: &str, name: &str) -> String {
    let key = name.strip_prefix(prefix).unwrap_or(name);
    let key = key.replace('-', "_").to_uppercase();
    format!("NVBES_{key}")
}

fn base64_decoded(encoded: &str) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_else(|| encoded.to_string())
}
