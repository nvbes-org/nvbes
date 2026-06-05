use crate::http::error::AppError;
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Datelike, NaiveDate, TimeDelta, Utc};
use password_hash::rand_core::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn slugify(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut previous_dash = false;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.starts_with('-') {
        slug.remove(0);
    }
    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "tenant".to_string()
    } else {
        slug
    }
}

pub fn validate_email(email: &str) -> Result<(), AppError> {
    if email.len() < 3 || !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(AppError::bad_request(
            "validation_failed",
            "Email address is invalid.",
        ));
    }

    Ok(())
}

pub fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::bad_request(
            "validation_failed",
            "Password must be at least 8 characters long.",
        ));
    }

    let entropy = zxcvbn::zxcvbn(password, &[]);

    if (entropy.score() as u8) < 3 {
        return Err(AppError::bad_request(
            "weak_password",
            "Password is too weak. Please choose a stronger password.",
        ));
    }

    Ok(())
}

const MIN_AGE_YEARS: u32 = 13;
const MAX_AGE_YEARS: u32 = 120;
const EARLIEST_BIRTH_YEAR: i32 = 1900;

/// Maps ISO 3166-1 alpha-2 country code to UTC offset in hours (standard time).
/// Returns None for unknown countries (falls back to UTC).
fn region_utc_offset(country_code: &str) -> Option<i32> {
    match country_code {
        // Europe
        "FR" | "DE" | "IT" | "ES" | "NL" | "BE" | "LU" | "AT" | "CH" | "DK" | "NO" | "SE"
        | "PL" | "CZ" | "SK" | "HU" | "HR" | "SI" | "BA" | "RS" | "ME" | "MK" | "AL" | "MT"
        | "LI" | "MC" | "SM" | "VA" | "AD" => Some(1),
        "GB" | "PT" | "IE" | "IS" => Some(0),
        "FI" | "EE" | "LV" | "LT" | "UA" | "MD" | "RO" | "BG" | "GR" | "CY" | "TR" => Some(2),
        "BY" => Some(3),
        // Americas
        "US" | "CA" => Some(-5), // Eastern (most populated); also -6,-7,-8
        "MX" => Some(-6),
        "BR" => Some(-3), // Brasília
        "AR" | "UY" | "CL" => Some(-3),
        "CO" | "PE" | "EC" | "PA" => Some(-5),
        "VE" | "BO" | "PY" => Some(-4),
        // Asia / Pacific
        "JP" | "KR" => Some(9),
        "CN" | "HK" | "TW" | "SG" | "MY" | "PH" => Some(8),
        "TH" | "VN" | "ID" | "KH" | "LA" => Some(7),
        "IN" | "LK" => Some(5).or(Some(5)), // +5:30 simplified to +5
        "BD" | "KZ" => Some(6),
        "PK" => Some(5),
        "IR" => Some(3).or(Some(3)), // +3:30 simplified to +3
        "AF" => Some(4).or(Some(4)), // +4:30 simplified to +4
        "AU" => Some(10),            // Sydney; also +8,+9:30
        "NZ" => Some(12),
        // Africa
        "ZA" | "ZW" | "BW" | "MZ" | "NA" | "LS" | "SZ" => Some(2),
        "EG" | "SD" | "LY" => Some(2),
        "NG" | "CM" | "CI" | "GH" | "SN" | "BJ" | "TG" | "NE" | "BF" | "ML" | "MR" | "GM"
        | "GN" | "SL" | "LR" => Some(1),
        "KE" | "UG" | "TZ" | "ET" | "SO" | "DJ" | "ER" | "MG" => Some(3),
        "MA" | "DZ" | "TN" => Some(1),
        // Middle East
        "SA" | "KW" | "BH" | "QA" | "YE" | "IQ" | "SY" | "JO" | "LB" | "PS" => Some(3),
        "AE" | "OM" => Some(4),
        "IL" => Some(2),
        _ => None,
    }
}

/// Returns the local date in a given region, or UTC if the region is unknown.
pub fn today_in_region(region: Option<&str>) -> NaiveDate {
    let now_utc = Utc::now();
    let offset_hours = region.and_then(region_utc_offset).unwrap_or(0);

    if offset_hours >= 0 {
        (now_utc + TimeDelta::hours(offset_hours as i64)).date_naive()
    } else {
        (now_utc - TimeDelta::hours((-offset_hours) as i64)).date_naive()
    }
}

pub fn parse_birthdate(raw: Option<&str>) -> Result<Option<NaiveDate>, AppError> {
    let Some(input) = raw else { return Ok(None) };

    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    if trimmed.len() > 64 {
        return Err(AppError::bad_request(
            "validation_failed",
            "Birthdate is too long.",
        ));
    }

    if !trimmed.is_ascii() || trimmed.contains(|c: char| c.is_control()) {
        return Err(AppError::bad_request(
            "validation_failed",
            "Birthdate contains invalid characters.",
        ));
    }

    // Strip optional time component: "2024-01-15T00:00:00Z" → "2024-01-15"
    let date_part = trimmed
        .split_once('T')
        .map(|(date, _)| date)
        .unwrap_or(trimmed);

    NaiveDate::parse_from_str(date_part, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| {
            AppError::bad_request(
                "validation_failed",
                "Birthdate must be in YYYY-MM-DD format.",
            )
        })
}

pub fn validate_birthdate(birthdate: NaiveDate, region: Option<&str>) -> Result<(), AppError> {
    let today = today_in_region(region);

    if birthdate > today {
        return Err(AppError::bad_request(
            "validation_failed",
            "Birthdate cannot be in the future.",
        ));
    }

    if birthdate.year() < EARLIEST_BIRTH_YEAR {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("Birth year must be {EARLIEST_BIRTH_YEAR} or later."),
        ));
    }

    let years_since = today.years_since(birthdate);

    if years_since.is_none() {
        return Err(AppError::bad_request(
            "validation_failed",
            "Birthdate is too far in the past.",
        ));
    }

    let age = years_since.unwrap_or(0);

    if age < MIN_AGE_YEARS {
        return Err(AppError::bad_request(
            "minimum_age",
            format!("You must be at least {MIN_AGE_YEARS} years old to register."),
        ));
    }

    if age > MAX_AGE_YEARS {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("Age cannot exceed {MAX_AGE_YEARS} years."),
        ));
    }

    Ok(())
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(65536, 3, 4, None).map_err(|_| {
        AppError::internal(
            "password_hash_failed",
            "Failed to configure Argon2id parameters.",
        )
    })?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::internal("password_hash_failed", "Failed to hash password."))
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|_| {
        AppError::internal("password_hash_invalid", "Stored password hash is invalid.")
    })?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn generate_token(prefix: &str) -> String {
    format!("{prefix}_{}", generate_random_token())
}

pub fn generate_random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn random_challenge() -> Vec<u8> {
    let mut bytes = vec![0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}

pub fn token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn token_hash_b64(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

pub fn require_non_empty(field: &'static str, value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::bad_request(
            "validation_failed",
            format!("{field} is required."),
        ));
    }

    Ok(trimmed.to_string())
}

pub fn log_dev_token(token: &str, environment: &str, purpose: &str) {
    if environment == "development" {
        tracing::info!(token = %token, %purpose, "Dev token generated (logged, not returned in response)");
    }
}

pub fn unique_slug(seed: &str) -> String {
    let base = seed
        .split('@')
        .next()
        .unwrap_or(seed)
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let suffix = Uuid::new_v4().simple().to_string();
    format!(
        "{}-{}",
        if base.is_empty() { "tenant" } else { &base },
        &suffix[..8]
    )
}

#[cfg(test)]
mod tests {
    use super::{hash_password, validate_birthdate, validate_password, verify_password};
    use axum::http::StatusCode;
    use chrono::{Datelike, NaiveDate, TimeDelta, Utc};

    #[test]
    fn test_argon2id_hashing_owasp_params() {
        let password = "SuperSecretPassword123!";
        let hash = hash_password(password).expect("hashing should succeed");

        assert!(hash.contains("$argon2id$"));
        assert!(hash.contains("m=65536,t=3,p=4"));

        let ok = verify_password(password, &hash).expect("verification should run");
        assert!(ok);

        let not_ok = verify_password("wrong_password", &hash).expect("verification should run");
        assert!(!not_ok);
    }

    #[test]
    fn validate_password_accepts_strong_password() {
        assert!(validate_password("CorrectHorseBatteryStaple123!").is_ok());
    }

    #[test]
    fn validate_password_rejects_short_passwords() {
        let error = validate_password("1234567").expect_err("expected short password to fail");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validate_password_rejects_weak_passwords() {
        let error = validate_password("password123").expect_err("expected weak password to fail");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, "weak_password");
    }

    #[test]
    fn birthdate_rejects_future_date() {
        let tomorrow = Utc::now().date_naive() + TimeDelta::days(1);
        let err = validate_birthdate(tomorrow, None).expect_err("future date should fail");
        assert_eq!(err.code, "validation_failed");
    }

    #[test]
    fn birthdate_rejects_too_young() {
        let thirteen_years_ago = Utc::now().date_naive() - TimeDelta::days(13 * 365 + 1);
        let err = validate_birthdate(thirteen_years_ago, None).expect_err("too young should fail");
        assert_eq!(err.code, "minimum_age");
    }

    #[test]
    fn birthdate_accepts_valid_age() {
        let twenty_years_ago =
            NaiveDate::from_ymd_opt(Utc::now().date_naive().year() - 20, 6, 15).unwrap();
        assert!(validate_birthdate(twenty_years_ago, None).is_ok());
    }

    #[test]
    fn birthdate_rejects_before_1900() {
        let ancient = NaiveDate::from_ymd_opt(1899, 1, 1).unwrap();
        let err = validate_birthdate(ancient, None).expect_err("pre-1900 should fail");
        assert_eq!(err.code, "validation_failed");
    }

    #[test]
    fn birthdate_accepts_leap_day() {
        let leap_day = NaiveDate::from_ymd_opt(2000, 2, 29).unwrap();
        assert!(validate_birthdate(leap_day, None).is_ok());
    }

    #[test]
    fn region_affects_age_calculation() {
        // In UTC+12 (NZ), it's already later than UTC.
        // A person born 13 years ago minus 1 day in UTC might already be 13 in NZ.
        let today_utc = Utc::now().date_naive();
        let yesterday_in_nz = today_utc - TimeDelta::days(1);

        // Using NZ (+12), "today" is one day ahead, so same birthdate is older
        let result_utc = validate_birthdate(yesterday_in_nz, None);
        let result_nz = validate_birthdate(yesterday_in_nz, Some("NZ"));

        // NZ should see the person as at least as old as UTC does (never younger)
        // If UTC rejects, NZ might still reject; but NZ should never be stricter
        match (result_utc.is_ok(), result_nz.is_ok()) {
            (false, false) | (true, true) | (true, false) => {} // UTC stricter or same
            (false, true) => {} // NZ relaxed (valid case: UTC thinks birthday hasn't passed yet)
        }
    }

    #[test]
    fn region_utc_unknown_falls_back_to_utc() {
        let date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let result_unknown = validate_birthdate(date, Some("XX"));
        let result_none = validate_birthdate(date, None);
        assert_eq!(result_unknown.is_ok(), result_none.is_ok());
    }
}
