use crate::http::error::AppError;
use chrono::{Datelike, NaiveDate, TimeDelta, Utc};

const MIN_AGE_YEARS: u32 = 13;
const MAX_AGE_YEARS: u32 = 120;
const EARLIEST_BIRTH_YEAR: i32 = 1900;

fn region_utc_offset(country_code: &str) -> Option<i32> {
    match country_code {
        "FR" | "DE" | "IT" | "ES" | "NL" | "BE" | "LU" | "AT" | "CH" | "DK" | "NO" | "SE"
        | "PL" | "CZ" | "SK" | "HU" | "HR" | "SI" | "BA" | "RS" | "ME" | "MK" | "AL" | "MT"
        | "LI" | "MC" | "SM" | "VA" | "AD" => Some(1),
        "GB" | "PT" | "IE" | "IS" => Some(0),
        "FI" | "EE" | "LV" | "LT" | "UA" | "MD" | "RO" | "BG" | "GR" | "CY" | "TR" => Some(2),
        "BY" => Some(3),
        "US" | "CA" => Some(-5),
        "MX" => Some(-6),
        "BR" => Some(-3),
        "AR" | "UY" | "CL" => Some(-3),
        "CO" | "PE" | "EC" | "PA" => Some(-5),
        "VE" | "BO" | "PY" => Some(-4),
        "JP" | "KR" => Some(9),
        "CN" | "HK" | "TW" | "SG" | "MY" | "PH" => Some(8),
        "TH" | "VN" | "ID" | "KH" | "LA" => Some(7),
        "IN" | "LK" => Some(5),
        "BD" | "KZ" => Some(6),
        "PK" => Some(5),
        "IR" => Some(3),
        "AF" => Some(4),
        "AU" => Some(10),
        "NZ" => Some(12),
        "ZA" | "ZW" | "BW" | "MZ" | "NA" | "LS" | "SZ" => Some(2),
        "EG" | "SD" | "LY" => Some(2),
        "NG" | "CM" | "CI" | "GH" | "SN" | "BJ" | "TG" | "NE" | "BF" | "ML" | "MR" | "GM"
        | "GN" | "SL" | "LR" => Some(1),
        "KE" | "UG" | "TZ" | "ET" | "SO" | "DJ" | "ER" | "MG" => Some(3),
        "MA" | "DZ" | "TN" => Some(1),
        "SA" | "KW" | "BH" | "QA" | "YE" | "IQ" | "SY" | "JO" | "LB" | "PS" => Some(3),
        "AE" | "OM" => Some(4),
        "IL" => Some(2),
        _ => None,
    }
}

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
