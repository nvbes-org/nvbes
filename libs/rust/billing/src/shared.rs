use chrono::{Datelike, NaiveDate, Utc};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

pub const EUR: &str = "EUR";
pub const STORAGE_OVERAGE_CENTS_PER_GB_MONTH: i64 = 4;
pub const EXTRA_SEAT_CENTS_PER_MONTH: i64 = 900;
pub const STRIPE_WEBHOOK_TOLERANCE_SECONDS: i64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum BillingRedirectUrlError {
    #[error("invalid_absolute_url")]
    InvalidAbsoluteUrl,
    #[error("invalid_origin")]
    InvalidOrigin,
}

pub fn current_billing_period() -> (NaiveDate, NaiveDate) {
    let today = Utc::now().date_naive();
    let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).expect("valid month start");
    let (next_year, next_month) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    let end = NaiveDate::from_ymd_opt(next_year, next_month, 1).expect("valid next month start");
    (start, end)
}

pub fn plan_monthly_price_cents(plan_code: &str) -> i64 {
    match plan_code {
        "solo_pro" => 1_500,
        "team" => 3_900,
        "team_plus" => 7_900,
        _ => 0,
    }
}

pub fn api_key_limit(plan_code: &str) -> i32 {
    match plan_code {
        "team_plus" => 20,
        "team" => 5,
        "solo_pro" => 1,
        _ => 0,
    }
}

pub fn validate_plan_code(plan_code: &str) -> Option<String> {
    let trimmed = plan_code.trim();
    if matches!(trimmed, "solo_pro" | "team" | "team_plus" | "trial") {
        Some(trimmed.to_owned())
    } else {
        None
    }
}

pub fn to_workspace_id(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

pub fn form_encode(fields: Vec<(String, String)>) -> String {
    fields
        .into_iter()
        .map(|(key, value)| format!("{}={}", percent_encode(&key), percent_encode(&value)))
        .collect::<Vec<_>>()
        .join("&")
}

pub fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

pub fn div_ceil(value: i64, divisor: i64) -> i64 {
    if value <= 0 {
        0
    } else {
        (value + divisor - 1) / divisor
    }
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn parse_uuid(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

pub fn resolve_billing_redirect_url(
    value: Option<&str>,
    default_url: &str,
    primary_origin: &str,
    staging_origin: Option<&str>,
) -> Result<String, BillingRedirectUrlError> {
    let candidate = value.unwrap_or(default_url).trim();
    let candidate_url =
        Url::parse(candidate).map_err(|_| BillingRedirectUrlError::InvalidAbsoluteUrl)?;

    let on_primary_origin = url_matches_allowed_origin(&candidate_url, primary_origin);
    let on_staging_origin =
        staging_origin.is_some_and(|origin| url_matches_allowed_origin(&candidate_url, origin));

    if !on_primary_origin && !on_staging_origin {
        return Err(BillingRedirectUrlError::InvalidOrigin);
    }

    Ok(candidate_url.to_string())
}

pub fn subscription_status_requires_lock(status: &str) -> bool {
    matches!(status, "past_due" | "canceled" | "suspended" | "incomplete")
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
    use super::{
        BillingRedirectUrlError, resolve_billing_redirect_url, subscription_status_requires_lock,
    };

    #[test]
    fn resolve_billing_redirect_url_accepts_allowed_origin() {
        let url = resolve_billing_redirect_url(
            Some("https://app.example.com/billing/success?workspace=1"),
            "https://app.example.com/billing/success",
            "https://app.example.com",
            Some("https://staging.example.com"),
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
        )
        .expect_err("expected external origin to be rejected");

        assert_eq!(err, BillingRedirectUrlError::InvalidOrigin);
    }

    #[test]
    fn subscription_status_requires_lock_blocks_degraded_states() {
        assert!(subscription_status_requires_lock("past_due"));
        assert!(subscription_status_requires_lock("canceled"));
        assert!(subscription_status_requires_lock("suspended"));
        assert!(subscription_status_requires_lock("incomplete"));
        assert!(!subscription_status_requires_lock("active"));
        assert!(!subscription_status_requires_lock("trialing"));
    }
}
