use super::AppConfig;

pub(crate) fn validate_request_e2ee(config: &AppConfig, strict_mode: bool) -> Result<(), String> {
    if config.request_e2ee_required && !config.request_e2ee_enabled {
        return Err(
            "NVBES_REQUEST_E2EE_ENABLED must be true when NVBES_REQUEST_E2EE_REQUIRED is true"
                .to_string(),
        );
    }
    if !config.request_e2ee_enabled {
        return Ok(());
    }

    let secret = config.request_e2ee_secret.as_deref().ok_or_else(|| {
        "NVBES_REQUEST_E2EE_SECRET is required when request E2EE is enabled".to_string()
    })?;

    if secret.len() < 32 {
        return Err("NVBES_REQUEST_E2EE_SECRET must be at least 32 characters long".to_string());
    }
    if config.request_e2ee_key_id.trim().is_empty() {
        return Err("NVBES_REQUEST_E2EE_KEY_ID must not be empty".to_string());
    }
    if strict_mode && config.request_e2ee_key_id == "default" {
        return Err(
            "NVBES_REQUEST_E2EE_KEY_ID cannot be 'default' outside development".to_string(),
        );
    }

    Ok(())
}
