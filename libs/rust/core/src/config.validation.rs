#[path = "config.validation.basics.rs"]
mod basics;
#[path = "config.validation.billing.rs"]
mod billing;
#[path = "config.validation.geo.rs"]
mod geo;
#[path = "config.validation.oauth.rs"]
mod oauth;
#[path = "config.validation.observability.rs"]
mod observability;
#[path = "config.validation.request_e2ee.rs"]
mod request_e2ee;
#[path = "config.validation.secrets.rs"]
mod secrets;
#[path = "config.validation.urls.rs"]
mod urls;

use super::AppConfig;

#[cfg(test)]
pub(crate) use basics::validate_positive_integer;
#[cfg(test)]
pub(crate) use geo::validate_ip_intelligence;
#[cfg(test)]
pub(crate) use observability::{
    validate_grafana_export_path, validate_observability_internal_token,
    validate_product_analytics, validate_profiling, validate_sentry,
};
#[cfg(test)]
pub(crate) use request_e2ee::validate_request_e2ee;
#[cfg(test)]
pub(crate) use secrets::validate_auth_factor_encryption;
#[cfg(test)]
pub(crate) use urls::{
    validate_jwt_secret, validate_profiling_endpoint, validate_public_url, validate_webauthn_rp_id,
};

pub(super) fn validate_config_urls_and_secrets(config: &AppConfig) -> Result<(), String> {
    let strict_mode = config.environment != "development";

    oauth::validate_fapi_high_assurance(config)?;

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
    urls::validate_public_url(
        "NVBES_MOLLIE_API_BASE_URL",
        &config.mollie_api_base_url,
        strict_mode,
        true,
    )?;
    urls::validate_public_url(
        "NVBES_TWILIO_API_BASE_URL",
        &config.twilio_api_base_url,
        strict_mode,
        true,
    )?;
    match config.otp_provider.as_str() {
        "mock" => {
            if strict_mode {
                return Err("NVBES_OTP_PROVIDER=mock is forbidden outside development".to_string());
            }
        }
        "twilio_verify" => {
            if config.twilio_account_sid.is_none() {
                return Err(
                    "NVBES_TWILIO_ACCOUNT_SID is required when NVBES_OTP_PROVIDER=twilio_verify"
                        .to_string(),
                );
            }
            if config.twilio_auth_token.is_none() {
                return Err(
                    "NVBES_TWILIO_AUTH_TOKEN is required when NVBES_OTP_PROVIDER=twilio_verify"
                        .to_string(),
                );
            }
            if config.twilio_verify_service_sid.is_none() {
                return Err("NVBES_TWILIO_VERIFY_SERVICE_SID is required when NVBES_OTP_PROVIDER=twilio_verify".to_string());
            }
        }
        provider => {
            return Err(format!(
                "Unsupported NVBES_OTP_PROVIDER={provider}. Supported values: mock, twilio_verify"
            ));
        }
    }
    if config.billing_mollie_enabled && config.mollie_api_key.is_none() {
        return Err(
            "NVBES_MOLLIE_API_KEY is required when NVBES_MOLLIE_ENABLED is true".to_string(),
        );
    }
    if config.billing_external_provider_fallback_enabled && config.stripe_secret_key.is_none() {
        return Err(
            "NVBES_STRIPE_SECRET_KEY is required when NVBES_BILLING_EXTERNAL_PROVIDER_FALLBACK_ENABLED is true"
                .to_string(),
        );
    }
    if config.billing_fraud_step_up_threshold > config.billing_fraud_manual_review_threshold
        || config.billing_fraud_manual_review_threshold > config.billing_fraud_block_threshold
        || config.billing_fraud_block_threshold > 100
    {
        return Err(
            "NVBES_BILLING_FRAUD thresholds must satisfy step_up <= manual_review <= block <= 100"
                .to_string(),
        );
    }
    billing::validate_billing_fraud_policy_overrides(config)?;
    urls::validate_database_url(&config.database_url, strict_mode)?;
    urls::validate_database_url(config.billing_database_url(), strict_mode)?;
    urls::validate_jwt_secret(&config.jwt_secret, strict_mode)?;
    observability::validate_grafana_export_path(config, strict_mode)?;
    observability::validate_product_analytics(config, strict_mode)?;
    observability::validate_sentry(config)?;
    observability::validate_profiling(config)?;
    observability::validate_observability_internal_token(config, strict_mode)?;
    geo::validate_maxmind_geolite(config)?;
    geo::validate_loyalsoldier_geoip(config)?;
    basics::validate_positive_integer(
        "NVBES_AUTH_SESSION_IDLE_TTL_MINUTES",
        config.auth_session_idle_ttl_minutes,
    )?;
    basics::validate_positive_integer(
        "NVBES_AUTH_VERIFICATION_RESEND_COOLDOWN_SECONDS",
        config.auth_verification_resend_cooldown_seconds,
    )?;
    basics::validate_positive_integer(
        "NVBES_AUTH_UNVERIFIED_ACCOUNT_TTL_DAYS",
        config.auth_unverified_account_ttl_days,
    )?;
    request_e2ee::validate_request_e2ee(config, strict_mode)?;
    secrets::validate_auth_factor_encryption(config, strict_mode)?;
    geo::validate_ip_intelligence(config, strict_mode)?;
    Ok(())
}
