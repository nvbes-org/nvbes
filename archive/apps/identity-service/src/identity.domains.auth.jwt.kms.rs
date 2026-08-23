use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::http::error::AppError;

#[derive(Clone)]
pub struct ScalewayKmsSigner {
    client: reqwest::Client,
    api_base_url: String,
    region: String,
    key_id: String,
    auth_token: String,
}

#[derive(Deserialize)]
struct PublicKeyResponse {
    pem: String,
}

#[derive(Deserialize)]
struct SignResponse {
    key_id: String,
    signature: String,
}

impl ScalewayKmsSigner {
    pub fn from_env() -> Result<Self, AppError> {
        let required = |name: &str| {
            std::env::var(name).map_err(|_| {
                AppError::internal(
                    "kms_configuration_invalid",
                    format!("{name} is required when NVBES_KMS_ENABLED=true"),
                )
            })
        };
        let region = std::env::var("NVBES_KMS_REGION").unwrap_or_else(|_| "fr-par".to_string());
        if !matches!(region.as_str(), "fr-par" | "nl-ams" | "pl-waw") {
            return Err(AppError::internal(
                "kms_configuration_invalid",
                "NVBES_KMS_REGION must be fr-par, nl-ams, or pl-waw.",
            ));
        }
        let auth_token = std::env::var("NVBES_KMS_AUTH_TOKEN")
            .or_else(|_| std::env::var("SCW_SECRET_KEY"))
            .map_err(|_| {
                AppError::internal(
                    "kms_configuration_invalid",
                    "NVBES_KMS_AUTH_TOKEN or SCW_SECRET_KEY is required when KMS is enabled.",
                )
            })?;
        let api_base_url = std::env::var("NVBES_KMS_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.scaleway.com".to_string());
        let client = reqwest::Client::builder()
            .https_only(api_base_url.starts_with("https://"))
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|error| AppError::internal("kms_client_failed", error.to_string()))?;
        Ok(Self {
            client,
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
            region,
            key_id: required("NVBES_KMS_KEY_ID")?,
            auth_token,
        })
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub async fn public_key_pem(&self) -> Result<String, AppError> {
        self.client
            .get(self.key_url("public-key"))
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .await
            .map_err(kms_transport_error)?
            .error_for_status()
            .map_err(kms_status_error)?
            .json::<PublicKeyResponse>()
            .await
            .map(|response| response.pem)
            .map_err(|error| AppError::internal("kms_invalid_response", error.to_string()))
    }

    pub async fn sign_ps256(&self, signing_input: &[u8]) -> Result<Vec<u8>, AppError> {
        let digest = STANDARD.encode(Sha256::digest(signing_input));
        let response = self
            .client
            .post(self.key_url("sign"))
            .header("X-Auth-Token", &self.auth_token)
            .json(&serde_json::json!({ "digest": digest }))
            .send()
            .await
            .map_err(kms_transport_error)?
            .error_for_status()
            .map_err(kms_status_error)?
            .json::<SignResponse>()
            .await
            .map_err(|error| AppError::internal("kms_invalid_response", error.to_string()))?;
        if response.key_id != self.key_id {
            return Err(AppError::internal(
                "kms_key_mismatch",
                "Scaleway KMS signed with an unexpected key identifier.",
            ));
        }
        STANDARD
            .decode(response.signature)
            .map_err(|error| AppError::internal("kms_invalid_signature", error.to_string()))
    }

    fn key_url(&self, operation: &str) -> String {
        format!(
            "{}/key-manager/v1alpha1/regions/{}/keys/{}/{}",
            self.api_base_url, self.region, self.key_id, operation
        )
    }
}

fn kms_transport_error(error: reqwest::Error) -> AppError {
    AppError::internal("kms_unavailable", error.to_string())
}

fn kms_status_error(error: reqwest::Error) -> AppError {
    AppError::internal("kms_request_rejected", error.to_string())
}
