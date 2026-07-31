use crate::http::error::AppError;

pub fn resolve_access_token_audience(
    audience: Option<&str>,
    resource_indicators: &[String],
) -> Result<String, AppError> {
    let audience = audience.map(str::trim).filter(|value| !value.is_empty());
    let resources = super::normalize_resources(resource_indicators.to_vec());

    match (audience, resources.as_slice()) {
        (Some(audience), []) => Ok(audience.to_string()),
        (None, [resource]) => Ok(resource.clone()),
        (Some(audience), [resource]) if audience == resource => Ok(audience.to_string()),
        (None, []) => Err(AppError::bad_request(
            "invalid_target",
            "An explicit audience or resource indicator is required.",
        )),
        (Some(_), [_]) => Err(AppError::bad_request(
            "invalid_target",
            "The audience and resource indicator must identify the same resource server.",
        )),
        (_, _) => Err(AppError::bad_request(
            "invalid_target",
            "Exactly one resource server may be targeted by an authorization request.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_access_token_audience;

    #[test]
    fn explicit_audience_is_the_access_token_target() {
        let audience = resolve_access_token_audience(Some("nvbes-account-service"), &[])
            .expect("audience should resolve");

        assert_eq!(audience, "nvbes-account-service");
    }

    #[test]
    fn one_resource_indicator_is_the_access_token_target() {
        let audience = resolve_access_token_audience(None, &["nvbes-cloud-service".to_string()])
            .expect("resource should resolve");

        assert_eq!(audience, "nvbes-cloud-service");
    }

    #[test]
    fn conflicting_audience_and_resource_are_rejected() {
        let error = resolve_access_token_audience(
            Some("nvbes-account-service"),
            &["nvbes-cloud-service".to_string()],
        )
        .expect_err("different resource servers must not be combined");

        assert_eq!(error.code, "invalid_target");
    }

    #[test]
    fn ambiguous_multi_resource_token_is_rejected() {
        let error = resolve_access_token_audience(
            None,
            &[
                "nvbes-account-service".to_string(),
                "nvbes-cloud-service".to_string(),
            ],
        )
        .expect_err("one access token must not span resource servers");

        assert_eq!(error.code, "invalid_target");
    }
}
