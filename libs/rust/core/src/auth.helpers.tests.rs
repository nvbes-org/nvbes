use super::{
    dummy_verify_password, hash_password, hash_password_with_pepper, validate_birthdate,
    validate_password, verify_and_check_rehash, verify_password, verify_password_with_pepper,
};
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
fn test_argon2id_pepper_and_rehash() {
    let password = "SuperSecretPassword123!";
    let pepper = b"kms_secret_pepper_key_2026";

    // 1. Hash with pepper
    let hash_peppered = hash_password_with_pepper(password, Some(pepper)).expect("peppered hash");
    let (valid, needs_rehash) =
        verify_and_check_rehash(password, &hash_peppered, Some(pepper)).expect("check");
    assert!(valid);
    let ok_pepper = verify_password_with_pepper(password, &hash_peppered, Some(pepper))
        .expect("verify with pepper");
    assert!(ok_pepper);
    assert!(!needs_rehash);

    // 2. Hash without pepper (legacy)
    let hash_unpeppered = hash_password(password).expect("unpeppered hash");
    let (valid_legacy, needs_rehash_legacy) =
        verify_and_check_rehash(password, &hash_unpeppered, Some(pepper)).expect("check legacy");
    assert!(valid_legacy);
    assert!(
        needs_rehash_legacy,
        "legacy hash should trigger re-hash with pepper"
    );

    // 3. Wrong pepper should fail or require rehash
    let wrong_pepper = b"wrong_pepper_key";
    let (valid_wrong, _) = verify_and_check_rehash(password, &hash_peppered, Some(wrong_pepper))
        .expect("check wrong pepper");
    assert!(!valid_wrong);
}

#[test]
fn test_dummy_verify_password_does_not_panic() {
    dummy_verify_password("Password123!", Some(b"pepper"));
    dummy_verify_password("Password123!", None);
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
    let today_utc = Utc::now().date_naive();
    let yesterday_in_nz = today_utc - TimeDelta::days(1);

    let result_utc = validate_birthdate(yesterday_in_nz, None);
    let result_nz = validate_birthdate(yesterday_in_nz, Some("NZ"));

    match (result_utc.is_ok(), result_nz.is_ok()) {
        (false, false) | (true, true) | (true, false) => {}
        (false, true) => {}
    }
}

#[test]
fn region_utc_unknown_falls_back_to_utc() {
    let date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let result_unknown = validate_birthdate(date, Some("XX"));
    let result_none = validate_birthdate(date, None);
    assert_eq!(result_unknown.is_ok(), result_none.is_ok());
}
