use base64::Engine;
use serde::Deserialize;

use super::AuditAnchorConfig;

#[derive(Debug, Deserialize)]
pub(super) struct KmsSignResponse {
    pub(super) key_id: String,
    pub(super) signature: String,
}

#[derive(Debug, Deserialize)]
struct KmsVerifyResponse {
    key_id: String,
    valid: bool,
}

pub(super) async fn sign_digest(
    config: &AuditAnchorConfig,
    digest: &[u8],
) -> anyhow::Result<KmsSignResponse> {
    let endpoint = format!(
        "https://api.scaleway.com/key-manager/v1alpha1/regions/{}/keys/{}/sign",
        config.region, config.kms_key_id
    );
    let response = reqwest::Client::new()
        .post(endpoint)
        .header("X-Auth-Token", &config.kms_auth_token)
        .json(&serde_json::json!({
            "digest": base64::engine::general_purpose::STANDARD.encode(digest),
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "audit anchor KMS signing failed with status {}",
            response.status()
        );
    }

    let signature: KmsSignResponse = response.json().await?;
    if signature.key_id != config.kms_key_id || signature.signature.trim().is_empty() {
        anyhow::bail!("audit anchor KMS returned an invalid signing response");
    }
    Ok(signature)
}

pub(super) async fn verify_signature(
    config: &AuditAnchorConfig,
    digest: &[u8],
    signature: &KmsSignResponse,
) -> anyhow::Result<()> {
    let endpoint = format!(
        "https://api.scaleway.com/key-manager/v1alpha1/regions/{}/keys/{}/verify",
        config.region, config.kms_key_id
    );
    let response = reqwest::Client::new()
        .post(endpoint)
        .header("X-Auth-Token", &config.kms_auth_token)
        .json(&serde_json::json!({
            "digest": base64::engine::general_purpose::STANDARD.encode(digest),
            "signature": &signature.signature,
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "audit anchor KMS verification failed with status {}",
            response.status()
        );
    }

    let verification: KmsVerifyResponse = response.json().await?;
    if verification.key_id != config.kms_key_id || !verification.valid {
        anyhow::bail!("audit anchor signature verification failed closed");
    }
    Ok(())
}
