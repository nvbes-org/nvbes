use crate::domains::federation::validation::validate_federation_endpoint_url_allowed;
use crate::http::error::AppError;

pub const EMAIL_NOT_VERIFIED: &str = "email_not_verified";
pub const TENANT_CONTEXT_REQUIRED: &str = "tenant_context_required";
pub const TENANT_MISMATCH: &str = "tenant_mismatch";
pub const PROVIDER_TYPE_MISMATCH: &str = "provider_type_mismatch";
pub const MISSING_EMAIL: &str = "missing_email";
pub const PROVIDER_INACTIVE: &str = "provider_inactive";
pub const INVALID_PROVIDER_ID: &str = "invalid_provider_id";
pub const DOMAIN_NOT_VERIFIED: &str = "domain_not_verified";
pub const DOMAIN_NOT_FOUND: &str = "domain_not_found";
pub const PROVIDER_NOT_FOUND: &str = "provider_not_found";
pub const CONNECTOR_NOT_FOUND: &str = "connector_not_found";
pub const IDENTITY_CONFLICT: &str = "identity_conflict";
pub const PRINCIPAL_CONFLICT: &str = "principal_conflict";
pub const INVALID_ID_TOKEN: &str = "invalid_id_token";
pub const JWKS_MISSING: &str = "jwks_missing";
pub const OIDC_DISCOVERY_INVALID: &str = "oidc_discovery_invalid";
pub const OIDC_DISCOVERY_FETCH_FAILED: &str = "oidc_discovery_fetch_failed";
pub const INVALID_SAML_RESPONSE: &str = "invalid_saml_response";
pub const SAML_METADATA_INVALID: &str = "saml_metadata_invalid";
pub const SAML_METADATA_FETCH_FAILED: &str = "saml_metadata_fetch_failed";
pub const SAML_SIGNATURE_INVALID: &str = "saml_signature_invalid";

pub fn normalize_scim_provider(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "The SCIM provider cannot be empty.",
        ));
    }

    Ok(value.to_string())
}

pub async fn normalize_scim_base_url(
    value: Option<&str>,
    strict_mode: bool,
) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            "The SCIM base_url cannot be empty.",
        ));
    }

    Ok(Some(
        validate_federation_endpoint_url_allowed(value, strict_mode).await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{normalize_scim_base_url, normalize_scim_provider};

    #[test]
    fn normalize_scim_provider_rejects_blank_values() {
        let err = normalize_scim_provider("   ").expect_err("blank provider should be rejected");
        assert_eq!(err.code, "validation_failed");
    }

    #[tokio::test]
    async fn normalize_scim_base_url_rejects_blank_values() {
        let err = normalize_scim_base_url(Some("   "), false)
            .await
            .expect_err("blank base_url should be rejected");
        assert_eq!(err.code, "validation_failed");
    }

    #[tokio::test]
    async fn normalize_scim_base_url_trims_and_validates() {
        let value = normalize_scim_base_url(Some(" https://scim.example.com/v2 "), false)
            .await
            .expect("base_url should be accepted");

        assert_eq!(value.as_deref(), Some("https://scim.example.com/v2"));
    }
}
