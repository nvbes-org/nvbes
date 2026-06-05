use crate::http::error::AppError;

pub fn validate_provider_configuration(
    provider_type: &str,
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
            if client_id.is_none() {
                return Err(AppError::bad_request(
                    "client_id_missing",
                    "The SAML provider requires a client_id.",
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
            validate_provider_configuration("oidc", None, Some("https://issuer.example"), None)
                .expect_err("expected missing client_id to be rejected");
        assert_eq!(err.code, "client_id_missing");

        let err = validate_provider_configuration("oidc", Some("client-1"), None, None)
            .expect_err("expected missing issuer and metadata_url to be rejected");
        assert_eq!(err.code, "provider_configuration_incomplete");
    }

    #[test]
    fn saml_provider_requires_all_core_fields() {
        let err = validate_provider_configuration(
            "saml",
            Some("sp-entity"),
            None,
            Some("https://metadata.example"),
        )
        .expect_err("expected missing issuer to be rejected");
        assert_eq!(err.code, "issuer_missing");

        let err = validate_provider_configuration(
            "saml",
            Some("sp-entity"),
            Some("https://issuer.example"),
            None,
        )
        .expect_err("expected missing metadata_url to be rejected");
        assert_eq!(err.code, "metadata_missing");
    }
}
