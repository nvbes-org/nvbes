use crate::http::error::AppError;
use nvbes_billing::BillingRedirectUrlError;

pub fn resolve_billing_redirect_url(
    value: Option<&str>,
    default_url: &str,
    primary_origin: &str,
    staging_origin: Option<&str>,
    field_name: &'static str,
    env_name: &'static str,
) -> Result<String, AppError> {
    nvbes_billing::resolve_billing_redirect_url(value, default_url, primary_origin, staging_origin)
        .map_err(|error| match error {
            BillingRedirectUrlError::InvalidAbsoluteUrl => AppError::bad_request(
                "invalid_billing_return_url",
                format!("{field_name} must be a valid absolute URL."),
            ),
            BillingRedirectUrlError::InvalidOrigin => AppError::bad_request(
                "invalid_billing_return_url",
                format!(
                    "{field_name} must stay on the configured application origin. Set {env_name} to an allowed URL."
                ),
            ),
        })
}
