use crate::http::error::AppError;

pub fn validate_provider_configuration(
    provider_type: &str,
    provider_family: &str,
    client_id: Option<&str>,
    issuer: Option<&str>,
    metadata_url: Option<&str>,
) -> Result<(), AppError> {
    let client_id = client_id.map(str::trim).filter(|value| !value.is_empty());
    let issuer = issuer.map(str::trim).filter(|value| !value.is_empty());
    let metadata_url = metadata_url
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match provider_type {
        "oidc" => {
            if !matches!(
                provider_family,
                "custom" | "google_workspace" | "azure_ad" | "okta"
            ) {
                return Err(AppError::bad_request(
                    "validation_failed",
                    "The OIDC provider family is invalid.",
                ));
            }

            if client_id.is_none() {
                return Err(AppError::bad_request(
                    "client_id_missing",
                    "The OIDC provider requires a client_id.",
                ));
            }

            if issuer.is_none() && metadata_url.is_none() {
                return Err(AppError::bad_request(
                    "provider_configuration_incomplete",
                    "The OIDC provider requires an issuer or metadata_url.",
                ));
            }
        }
        "saml" => {
            if provider_family != "custom" {
                return Err(AppError::bad_request(
                    "validation_failed",
                    "SAML providers must use the custom provider family.",
                ));
            }

            if client_id.is_none() {
                return Err(AppError::bad_request(
                    "client_id_missing",
                "The SAML provider requires an SP entity ID in client_id.",
                ));
            }

            if issuer.is_none() {
                return Err(AppError::bad_request(
                    "issuer_missing",
                    "The SAML provider requires an issuer.",
                ));
            }

            if metadata_url.is_none() {
                return Err(AppError::bad_request(
                    "metadata_missing",
                    "The SAML provider requires a metadata_url.",
                ));
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_provider_configuration;

    #[test]
    fn oidc_provider_requires_client_id_and_source() {
        let err =
            validate_provider_configuration(
                "oidc",
                "google_workspace",
                None,
                Some("https://issuer.example"),
                None,
            )
                .expect_err("expected missing client_id to be rejected");
        assert_eq!(err.code, "client_id_missing");

        let err =
            validate_provider_configuration("oidc", "azure_ad", Some("client-1"), None, None)
            .expect_err("expected missing issuer and metadata_url to be rejected");
        assert_eq!(err.code, "provider_configuration_incomplete");
    }

    #[test]
    fn saml_provider_requires_all_core_fields() {
        let err = validate_provider_configuration(
            "saml",
            "custom",
            Some("sp-entity"),
            None,
            Some("https://metadata.example"),
        )
        .expect_err("expected missing issuer to be rejected");
        assert_eq!(err.code, "issuer_missing");

        let err = validate_provider_configuration(
            "saml",
            "custom",
            Some("sp-entity"),
            Some("https://issuer.example"),
            None,
        )
        .expect_err("expected missing metadata_url to be rejected");
        assert_eq!(err.code, "metadata_missing");
    }

    #[test]
    fn saml_provider_rejects_oidc_provider_families() {
        let err = validate_provider_configuration(
            "saml",
            "okta",
            Some("sp-entity"),
            Some("https://issuer.example"),
            Some("https://metadata.example"),
        )
        .expect_err("expected SAML provider family to be rejected");

        assert_eq!(err.code, "validation_failed");
    }
}
