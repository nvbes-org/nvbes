use base64::Engine;

use super::AppConfig;

pub(crate) fn validate_auth_factor_encryption(
    config: &AppConfig,
    strict_mode: bool,
) -> Result<(), String> {
    if config.auth_factor_encryption_key_version == 0 {
        return Err(
            "NVBES_AUTH_FACTOR_ENCRYPTION_KEY_VERSION must be greater than zero".to_string(),
        );
    }

    let Some(encoded_key) = config.auth_factor_encryption_key.as_deref() else {
        if strict_mode {
            return Err(
                "NVBES_AUTH_FACTOR_ENCRYPTION_KEY is required outside development".to_string(),
            );
        }
        return Ok(());
    };

    let key = base64::engine::general_purpose::STANDARD
        .decode(encoded_key)
        .map_err(|_| "NVBES_AUTH_FACTOR_ENCRYPTION_KEY must be valid base64".to_string())?;
    if key.len() < 32 {
        return Err(
            "NVBES_AUTH_FACTOR_ENCRYPTION_KEY must decode to at least 32 bytes".to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
#[path = "config.validation.secrets.tests.rs"]
mod tests;
