use reqwest::Url;
use std::net::IpAddr;
use tokio::net::lookup_host;

use super::http::is_public_ip;
use crate::http::error::AppError;

pub fn normalize_domain(input: &str) -> Result<String, AppError> {
    let value = input.trim().trim_end_matches('.').to_ascii_lowercase();
    validate_domain(&value)?;
    Ok(value)
}

pub fn validate_domain(domain: &str) -> Result<(), AppError> {
    if domain.is_empty() || domain.len() > 253 || domain.starts_with('.') || domain.ends_with('.') {
        return Err(AppError::bad_request(
            "validation_failed",
            "The domain name is invalid.",
        ));
    }

    if domain.contains("..") {
        return Err(AppError::bad_request(
            "validation_failed",
            "The domain name is invalid.",
        ));
    }

    let is_valid = domain.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    });

    if !is_valid {
        return Err(AppError::bad_request(
            "validation_failed",
            "The domain name is invalid.",
        ));
    }

    Ok(())
}

pub fn normalize_federated_provider_type(input: &str) -> Result<String, AppError> {
    let value = input.trim().to_ascii_lowercase();
    match value.as_str() {
        "password" | "oidc" | "saml" | "webauthn" | "passkey" | "recovery_code" => Ok(value),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "The provider type is invalid.",
        )),
    }
}

pub fn normalize_registry_status(status: Option<&str>) -> Result<String, AppError> {
    let value = status.unwrap_or("active").trim().to_ascii_lowercase();
    match value.as_str() {
        "active" | "suspended" | "disabled" | "pending" | "pending_approval" => Ok(value),
        _ => Err(AppError::bad_request(
            "validation_failed",
            "The status value is invalid.",
        )),
    }
}

pub fn opt_trimmed(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub fn validate_federation_endpoint_url(value: &str) -> Result<Url, AppError> {
    let url = Url::parse(value.trim()).map_err(|_| {
        AppError::bad_request(
            "validation_failed",
            "The federation endpoint URL is invalid.",
        )
    })?;

    if url.username() != "" || url.password().is_some() {
        return Err(AppError::bad_request(
            "validation_failed",
            "The federation endpoint URL cannot contain credentials.",
        ));
    }

    if url.host_str().is_none() {
        return Err(AppError::bad_request(
            "validation_failed",
            "The federation endpoint URL must include a host.",
        ));
    }

    Ok(url)
}

pub async fn validate_federation_endpoint_url_allowed(
    value: &str,
    strict_mode: bool,
) -> Result<String, AppError> {
    let url = validate_federation_endpoint_url(value)?;
    validate_federation_url_policy(&url, strict_mode).await?;
    Ok(url.to_string())
}

async fn validate_federation_url_policy(url: &Url, strict_mode: bool) -> Result<(), AppError> {
    if !matches!(url.scheme(), "https") && strict_mode {
        return Err(AppError::bad_request(
            "validation_failed",
            "Federation endpoints must use HTTPS in non-development environments.",
        ));
    }

    if !strict_mode {
        return Ok(());
    }

    let host = url.host_str().ok_or_else(|| {
        AppError::bad_request(
            "validation_failed",
            "The federation endpoint URL must include a host.",
        )
    })?;

    let port = url.port_or_known_default().ok_or_else(|| {
        AppError::bad_request(
            "validation_failed",
            "The federation endpoint URL must include a port or a known scheme.",
        )
    })?;

    if let Ok(ip_addr) = host.parse::<IpAddr>() {
        if !is_public_ip(ip_addr) {
            return Err(AppError::bad_request(
                "validation_failed",
                "Federation endpoints cannot target private or loopback addresses.",
            ));
        }
        return Ok(());
    }

    let resolved = lookup_host((host, port)).await.map_err(|_| {
        AppError::bad_request(
            "validation_failed",
            "The federation endpoint host could not be resolved.",
        )
    })?;

    let mut saw_address = false;
    for address in resolved {
        saw_address = true;
        if !is_public_ip(address.ip()) {
            return Err(AppError::bad_request(
                "validation_failed",
                "Federation endpoints cannot resolve to private or loopback addresses.",
            ));
        }
    }

    if !saw_address {
        return Err(AppError::bad_request(
            "validation_failed",
            "The federation endpoint host could not be resolved.",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_domain, normalize_federated_provider_type, normalize_registry_status,
        opt_trimmed, validate_federation_endpoint_url, validate_federation_endpoint_url_allowed,
    };

    #[test]
    fn normalize_domain_trims_and_lowercases() {
        assert_eq!(
            normalize_domain(" IdP.Example.COM. ").expect("domain should normalize"),
            "idp.example.com"
        );
    }

    #[test]
    fn normalize_federated_provider_type_accepts_supported_values() {
        assert_eq!(
            normalize_federated_provider_type(" OIDC ").expect("provider type should normalize"),
            "oidc"
        );
    }

    #[test]
    fn normalize_registry_status_defaults_to_active_and_trims() {
        assert_eq!(
            normalize_registry_status(None).expect("default status should be active"),
            "active"
        );
        assert_eq!(
            normalize_registry_status(Some(" Pending ")).expect("status should normalize"),
            "pending"
        );
    }

    #[test]
    fn opt_trimmed_discards_blank_values() {
        assert_eq!(opt_trimmed(Some("  ".to_string())), None);
        assert_eq!(
            opt_trimmed(Some(" https://metadata.example ".to_string())),
            Some("https://metadata.example".to_string())
        );
    }

    #[test]
    fn validate_federation_endpoint_url_rejects_credentials() {
        let err = validate_federation_endpoint_url("https://user:pass@example.com")
            .expect_err("expected credentials to be rejected");
        assert_eq!(err.code, "validation_failed");
    }

    #[test]
    fn validate_federation_endpoint_url_rejects_missing_host() {
        let err = validate_federation_endpoint_url("https://")
            .expect_err("expected missing host to be rejected");

        assert_eq!(err.code, "validation_failed");
    }

    #[tokio::test]
    async fn validate_federation_endpoint_url_allows_public_https_in_strict_mode() {
        let url = validate_federation_endpoint_url_allowed("https://1.1.1.1/metadata", true)
            .await
            .expect("expected public HTTPS URL to be accepted");
        assert_eq!(url, "https://1.1.1.1/metadata");
    }

    #[tokio::test]
    async fn validate_federation_endpoint_url_rejects_loopback_in_strict_mode() {
        let err = validate_federation_endpoint_url_allowed("https://localhost/metadata", true)
            .await
            .expect_err("expected loopback URL to be rejected");
        assert_eq!(err.code, "validation_failed");
    }
}
