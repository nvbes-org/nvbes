use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct KmsConfig {
    pub access_key: String,
    pub secret_key: String,
    pub project_id: String,
    pub region: String,
    pub key_id: Option<String>,
    pub enabled: bool,
}

impl KmsConfig {
    pub fn from_env() -> Option<Self> {
        let enabled = std::env::var("NVBES_KMS_ENABLED")
            .ok()
            .map(|v| v == "true")
            .unwrap_or(false);

        if !enabled {
            return None;
        }

        Some(Self {
            access_key: std::env::var("SCW_ACCESS_KEY").ok()?,
            secret_key: std::env::var("SCW_SECRET_KEY").ok()?,
            project_id: std::env::var("SCW_DEFAULT_PROJECT_ID")
                .or_else(|_| std::env::var("SCW_PROJECT_ID"))
                .ok()?,
            region: std::env::var("SCW_DEFAULT_REGION")
                .or_else(|_| std::env::var("SCW_REGION"))
                .ok()
                .unwrap_or_else(|| "fr-par".to_string()),
            key_id: std::env::var("SCW_KMS_KEY_ID").ok(),
            enabled: true,
        })
    }

    pub fn api_base(&self) -> String {
        "https://api.scaleway.com".to_string()
    }
}

#[derive(Debug, Serialize)]
struct CreateKeyRequest {
    project_id: String,
    name: String,
    usage: KeyUsage,
    description: String,
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rotation_policy: Option<RotationPolicy>,
}

#[derive(Debug, Serialize)]
struct KeyUsage {
    #[serde(rename = "asymmetric_signing")]
    asymmetric_signing: String,
}

#[derive(Debug, Serialize)]
struct RotationPolicy {
    rotation_period: String,
}

#[derive(Debug, Deserialize)]
pub struct KmsKey {
    pub id: String,
    pub name: String,
    pub state: String,
    pub rotation_count: u32,
    #[serde(rename = "rotated_at")]
    pub rotated_at: Option<String>,
    pub created_at: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
struct SignRequest {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SignResponse {
    pub signature: String,
    pub key_id: String,
}

#[derive(Debug, Deserialize)]
pub struct GetPublicKeyResponse {
    pub public_key: String,
    pub key_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ListKeysResponse {
    pub keys: Vec<KmsKey>,
    pub total_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct RotateKeyResponse {
    pub id: String,
    pub rotation_count: u32,
    pub rotated_at: String,
}

pub struct KmsClient {
    http: Client,
    config: KmsConfig,
}

impl KmsClient {
    pub fn new(config: KmsConfig) -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("valid HTTP client"),
            config,
        }
    }

    pub fn config(&self) -> &KmsConfig {
        &self.config
    }

    pub async fn create_key(&self, name: &str) -> Result<KmsKey, KmsError> {
        let req = CreateKeyRequest {
            project_id: self.config.project_id.clone(),
            name: name.to_string(),
            usage: KeyUsage {
                asymmetric_signing: "rsa_pkcs1_sha256".to_string(),
            },
            description: "nvbes JWT signing key".to_string(),
            tags: vec!["nvbes".to_string(), "jwt".to_string()],
            rotation_policy: Some(RotationPolicy {
                rotation_period: "P30D".to_string(),
            }),
        };

        let url = format!(
            "{}/key-manager/v1alpha1/regions/{}/keys",
            self.config.api_base(),
            self.config.region
        );

        let resp = self
            .http
            .post(&url)
            .header("X-Auth-Token", &self.config.secret_key)
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(KmsError::ApiError { status: 0, body });
        }

        let key: KmsKey = resp.json().await?;
        Ok(key)
    }

    pub async fn rotate_key(&self, key_id: &str) -> Result<RotateKeyResponse, KmsError> {
        let url = format!(
            "{}/key-manager/v1alpha1/regions/{}/keys/{}/rotate",
            self.config.api_base(),
            self.config.region,
            key_id
        );

        let resp = self
            .http
            .post(&url)
            .header("X-Auth-Token", &self.config.secret_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "key_id": key_id }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(KmsError::ApiError { status: 0, body });
        }

        let result: RotateKeyResponse = resp.json().await?;
        Ok(result)
    }

    pub async fn sign(&self, key_id: &str, message: &[u8]) -> Result<SignResponse, KmsError> {
        let req = SignRequest {
            message: base64::engine::general_purpose::STANDARD.encode(message),
            message_type: Some("digest".to_string()),
        };

        let url = format!(
            "{}/key-manager/v1alpha1/regions/{}/keys/{}/sign",
            self.config.api_base(),
            self.config.region,
            key_id
        );

        let resp = self
            .http
            .post(&url)
            .header("X-Auth-Token", &self.config.secret_key)
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(KmsError::ApiError { status: 0, body });
        }

        let result: SignResponse = resp.json().await?;
        Ok(result)
    }

    pub async fn get_public_key(&self, key_id: &str) -> Result<GetPublicKeyResponse, KmsError> {
        let url = format!(
            "{}/key-manager/v1alpha1/regions/{}/keys/{}/public_key",
            self.config.api_base(),
            self.config.region,
            key_id
        );

        let resp = self
            .http
            .get(&url)
            .header("X-Auth-Token", &self.config.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(KmsError::ApiError { status: 0, body });
        }

        let result: GetPublicKeyResponse = resp.json().await?;
        Ok(result)
    }

    pub async fn list_keys(&self) -> Result<ListKeysResponse, KmsError> {
        let url = format!(
            "{}/key-manager/v1alpha1/regions/{}/keys?project_id={}&usage=asymmetric_signing",
            self.config.api_base(),
            self.config.region,
            self.config.project_id
        );

        let resp = self
            .http
            .get(&url)
            .header("X-Auth-Token", &self.config.secret_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(KmsError::ApiError { status: 0, body });
        }

        let result: ListKeysResponse = resp.json().await?;
        Ok(result)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KmsError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("KMS API error (status {status}): {body}")]
    ApiError { status: u16, body: String },
    #[error("key not found: {0}")]
    KeyNotFound(String),
    #[error("signing failed: {0}")]
    SigningFailed(String),
}
