use chrono::{Datelike, NaiveDate, Utc};
use uuid::Uuid;

pub const EUR: &str = "EUR";
pub const STORAGE_OVERAGE_CENTS_PER_GB_MONTH: i64 = 4;
pub const EXTRA_SEAT_CENTS_PER_MONTH: i64 = 900;
pub const STRIPE_WEBHOOK_TOLERANCE_SECONDS: i64 = 300;

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
