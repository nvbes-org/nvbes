#[path = "config.validation.basics.rs"]
mod basics;
#[path = "config.validation.observability.rs"]
mod observability;
#[path = "config.validation.request_e2ee.rs"]
mod request_e2ee;
#[path = "config.validation.urls.rs"]
mod urls;

use super::AppConfig;

#[cfg(test)]
pub(crate) use basics::validate_positive_integer;
#[cfg(test)]
pub(crate) use observability::{
    validate_grafana_export_path, validate_observability_internal_token,
    validate_posthog_analytics, validate_profiling,
};
#[cfg(test)]
pub(crate) use request_e2ee::validate_request_e2ee;
#[cfg(test)]
pub(crate) use urls::{
    validate_jwt_secret, validate_profiling_endpoint, validate_public_url, validate_webauthn_rp_id,
};

pub(super) fn validate_config_urls_and_secrets(config: &AppConfig) -> Result<(), String> {
    let strict_mode = config.environment != "development";

    if config.mtls_enabled {
        if config.tls_cert_path.as_deref().unwrap_or("").is_empty() {
            return Err(
                "NVBES_TLS_CERT_PATH is required when NVBES_MTLS_ENABLED is true".to_string(),
            );
        }
        if config.tls_key_path.as_deref().unwrap_or("").is_empty() {
            return Err(
                "NVBES_TLS_KEY_PATH is required when NVBES_MTLS_ENABLED is true".to_string(),
            );
        }
        if config
            .tls_client_ca_path
            .as_deref()
            .unwrap_or("")
            .is_empty()
        {
            return Err(
                "NVBES_TLS_CLIENT_CA_PATH is required when NVBES_MTLS_ENABLED is true".to_string(),
            );
        }
    }

    urls::validate_public_url(
        "NVBES_WEB_BASE_URL",
        &config.web_base_url,
        strict_mode,
        true,
    )?;
    urls::validate_public_url(
        "NVBES_API_BASE_URL",
        &config.api_base_url,
        strict_mode,
        true,
    )?;
    urls::validate_public_url(
        "NVBES_BILLING_SUCCESS_URL",
        &config.billing_default_success_url,
        strict_mode,
        true,
    )?;
    urls::validate_public_url(
        "NVBES_BILLING_CANCEL_URL",
        &config.billing_default_cancel_url,
        strict_mode,
        true,
    )?;
    urls::validate_public_url(
        "NVBES_BILLING_PORTAL_RETURN_URL",
        &config.billing_default_portal_return_url,
        strict_mode,
        true,
    )?;
    urls::validate_public_url(
        "NVBES_WEBAUTHN_RP_ORIGIN",
        &config.webauthn_rp_origin,
        strict_mode,
        true,
    )?;
    urls::validate_webauthn_rp_id(&config.webauthn_rp_id, strict_mode)?;
    urls::validate_public_url(
        "NVBES_STRIPE_API_BASE_URL",
        &config.stripe_api_base_url,
        strict_mode,
        true,
    )?;
    urls::validate_database_url(&config.database_url, strict_mode)?;
    urls::validate_jwt_secret(&config.jwt_secret, strict_mode)?;
    observability::validate_grafana_export_path(config, strict_mode)?;
    observability::validate_posthog_analytics(config, strict_mode)?;
    observability::validate_profiling(config)?;
    observability::validate_observability_internal_token(config, strict_mode)?;
    basics::validate_positive_integer(
        "NVBES_AUTH_VERIFICATION_RESEND_COOLDOWN_SECONDS",
        config.auth_verification_resend_cooldown_seconds,
    )?;
    basics::validate_positive_integer(
        "NVBES_AUTH_UNVERIFIED_ACCOUNT_TTL_DAYS",
        config.auth_unverified_account_ttl_days,
    )?;
    request_e2ee::validate_request_e2ee(config, strict_mode)?;
    Ok(())
}
