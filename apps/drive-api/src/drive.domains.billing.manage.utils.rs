use crate::http::error::AppError;
use reqwest::Url;

pub fn resolve_billing_redirect_url(
    value: Option<&str>,
    default_url: &str,
    primary_origin: &str,
    staging_origin: Option<&str>,
    field_name: &'static str,
    env_name: &'static str,
) -> Result<String, AppError> {
    let candidate = value.unwrap_or(default_url).trim();
    let candidate_url = Url::parse(candidate).map_err(|_| {
        AppError::bad_request(
            "invalid_billing_return_url",
            format!("{field_name} must be a valid absolute URL."),
        )
    })?;

    let on_primary_origin = url_matches_allowed_origin(&candidate_url, primary_origin);
    let on_staging_origin =
        staging_origin.is_some_and(|origin| url_matches_allowed_origin(&candidate_url, origin));

    if !on_primary_origin && !on_staging_origin {
        return Err(AppError::bad_request(
            "invalid_billing_return_url",
            format!(
                "{field_name} must stay on the configured application origin. Set {env_name} to an allowed URL."
            ),
        ));
    }

    Ok(candidate_url.to_string())
}

fn url_matches_allowed_origin(candidate: &Url, allowed: &str) -> bool {
    let allowed = match Url::parse(allowed.trim()) {
        Ok(url) => url,
        Err(_) => return false,
    };

    candidate.scheme() == allowed.scheme()
        && candidate.host_str() == allowed.host_str()
        && candidate.port_or_known_default() == allowed.port_or_known_default()
}

#[cfg(test)]
mod tests {
    use super::resolve_billing_redirect_url;

    #[test]
    fn resolve_billing_redirect_url_accepts_allowed_origin() {
        let url = resolve_billing_redirect_url(
            Some("https://app.example.com/billing/success?workspace=1"),
            "https://app.example.com/billing/success",
            "https://app.example.com",
            Some("https://staging.example.com"),
            "success_url",
            "NVBES_BILLING_SUCCESS_URL",
        )
        .expect("expected allowed origin to be accepted");

        assert_eq!(url, "https://app.example.com/billing/success?workspace=1");
    }

    #[test]
    fn resolve_billing_redirect_url_rejects_external_origin() {
        let err = resolve_billing_redirect_url(
            Some("https://evil.example/phish"),
            "https://app.example.com/billing/success",
            "https://app.example.com",
            Some("https://staging.example.com"),
            "success_url",
            "NVBES_BILLING_SUCCESS_URL",
        )
        .expect_err("expected external origin to be rejected");

        assert_eq!(err.code, "invalid_billing_return_url");
    }
}
