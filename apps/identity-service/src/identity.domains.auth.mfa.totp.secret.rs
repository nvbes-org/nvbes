use base64::Engine;
use nvbes_core::config::AppConfig;
use nvbes_storage::{EnvelopeEncryptedData, decrypt_envelope, encrypt_envelope};
use uuid::Uuid;

use crate::http::error::AppError;

const DEVELOPMENT_KEY: &[u8; 32] = b"nvbes-development-factor-key-v1!";

pub(super) fn encrypt(
    config: &AppConfig,
    tenant_id: Uuid,
    principal_id: Uuid,
    factor_id: Uuid,
    secret: &str,
) -> Result<serde_json::Value, AppError> {
    let context = encryption_context(tenant_id, principal_id, factor_id);
    let envelope = encrypt_envelope(
        &master_key(config)?,
        &context,
        config.auth_factor_encryption_key_version,
        secret.as_bytes(),
    )
    .map_err(|error| AppError::internal("totp_secret_encryption_failed", error.to_string()))?;
    serde_json::to_value(envelope)
        .map_err(|error| AppError::internal("totp_secret_encoding_failed", error.to_string()))
}

pub(super) fn decrypt(
    config: &AppConfig,
    tenant_id: Uuid,
    principal_id: Uuid,
    factor_id: Uuid,
    envelope: serde_json::Value,
) -> Result<String, AppError> {
    let envelope = serde_json::from_value::<EnvelopeEncryptedData>(envelope)
        .map_err(|error| AppError::internal("totp_secret_envelope_invalid", error.to_string()))?;
    let context = encryption_context(tenant_id, principal_id, factor_id);
    let plaintext = decrypt_envelope(&master_key(config)?, &context, &envelope)
        .map_err(|error| AppError::internal("totp_secret_decryption_failed", error.to_string()))?;
    String::from_utf8(plaintext)
        .map_err(|error| AppError::internal("totp_secret_encoding_invalid", error.to_string()))
}

fn master_key(config: &AppConfig) -> Result<Vec<u8>, AppError> {
    match config.auth_factor_encryption_key.as_deref() {
        Some(encoded) => {
            let key = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|_| {
                    AppError::internal(
                        "factor_encryption_key_invalid",
                        "NVBES_AUTH_FACTOR_ENCRYPTION_KEY must be valid base64.",
                    )
                })?;
            if key.len() < 32 {
                return Err(AppError::internal(
                    "factor_encryption_key_invalid",
                    "NVBES_AUTH_FACTOR_ENCRYPTION_KEY must decode to at least 32 bytes.",
                ));
            }
            Ok(key)
        }
        None if config.environment == "development" || config.environment == "test" => {
            Ok(DEVELOPMENT_KEY.to_vec())
        }
        None => Err(AppError::internal(
            "factor_encryption_key_missing",
            "NVBES_AUTH_FACTOR_ENCRYPTION_KEY is required outside development.",
        )),
    }
}

fn encryption_context(tenant_id: Uuid, principal_id: Uuid, factor_id: Uuid) -> String {
    format!("totp:{tenant_id}:{principal_id}:{factor_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AppConfig {
        AppConfig {
            environment: "test".to_string(),
            auth_factor_encryption_key_version: 1,
            ..AppConfig::default()
        }
    }

    #[test]
    fn envelope_round_trip_is_bound_to_factor_context() {
        let tenant_id = Uuid::new_v4();
        let principal_id = Uuid::new_v4();
        let factor_id = Uuid::new_v4();
        let envelope = encrypt(
            &config(),
            tenant_id,
            principal_id,
            factor_id,
            "JBSWY3DPEHPK3PXP",
        )
        .expect("secret should encrypt");

        assert!(!envelope.to_string().contains("JBSWY3DPEHPK3PXP"));
        assert_eq!(
            decrypt(
                &config(),
                tenant_id,
                principal_id,
                factor_id,
                envelope.clone()
            )
            .expect("secret should decrypt"),
            "JBSWY3DPEHPK3PXP"
        );
        assert!(decrypt(&config(), tenant_id, principal_id, Uuid::new_v4(), envelope).is_err());
    }
}
