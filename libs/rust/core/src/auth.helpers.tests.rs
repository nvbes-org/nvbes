use super::birthdate::{date_in_region_at, validate_birthdate_on};
use super::{
    dummy_verify_password, hash_password, hash_password_with_pepper, validate_password,
    verify_and_check_rehash, verify_password, verify_password_with_pepper,
};
use axum::http::StatusCode;
use chrono::{NaiveDate, TimeZone, Utc};

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 12).unwrap()
}

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
fn password_verify_table_covers_invalid_hash_and_mismatch() {
    let password = "SuperSecretPassword123!";
    let hash = hash_password(password).expect("hash");

    let cases = [
        ("not-a-phc-hash", "err"),
        ("$argon2id$v=19$m=65536,t=3,p=4$bad", "err"),
        ("wrong-password", "false"),
        (password, "true"),
    ];
    for (candidate, expected) in cases {
        let result = if expected == "err" {
            verify_password(password, candidate)
        } else {
            verify_password(candidate, &hash)
        };
        match expected {
            "err" => {
                let err = result.expect_err("invalid hash");
                assert_eq!(err.code, "password_hash_invalid");
            }
            "false" => assert!(!result.expect("verify")),
            "true" => assert!(result.expect("verify")),
            _ => unreachable!(),
        }
    }
}

#[test]
fn empty_pepper_is_treated_as_unpeppered() {
    let password = "SuperSecretPassword123!";
    let hash = hash_password_with_pepper(password, Some(&[])).expect("empty pepper hash");
    let (valid, needs_rehash) =
        verify_and_check_rehash(password, &hash, Some(&[])).expect("verify empty pepper");
    assert!(valid);
    assert!(!needs_rehash);
    assert!(verify_password(password, &hash).expect("unpeppered verify against empty-pepper hash"));
}

#[test]
fn weak_argon2_params_require_rehash() {
    use argon2::password_hash::PasswordHasher;
    use argon2::{Algorithm, Argon2, Params, Version};

    let password = "SuperSecretPassword123!";
    let params = Params::new(4096, 1, 1, None).expect("weak params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let weak_hash = argon2
        .hash_password(password.as_bytes())
        .expect("weak hash")
        .to_string();

    let (valid, needs_rehash) =
        verify_and_check_rehash(password, &weak_hash, None).expect("weak verify");
    assert!(valid);
    assert!(needs_rehash, "weaker params must request rehash");
}

#[test]
fn validate_password_accepts_strong_password() {
    assert!(validate_password("CorrectHorseBatteryStaple123!").is_ok());
}

#[test]
fn validate_password_rejects_short_passwords() {
    let error = validate_password("short").expect_err("expected short password to fail");
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
}

#[test]
fn validate_password_rejects_weak_passwords() {
    let error = validate_password("passwordpassword").expect_err("expected weak password to fail");
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.code, "weak_password");
}

#[test]
fn validate_password_accepts_long_passphrases_without_composition_rules() {
    assert!(validate_password("rivière nuage cuivre galaxie").is_ok());
}

#[test]
fn validate_password_counts_unicode_characters() {
    let error = validate_password("é".repeat(7).as_str())
        .expect_err("seven Unicode characters should be too short");
    assert_eq!(error.code, "validation_failed");
}

#[test]
fn validate_password_rejects_values_above_supported_limit() {
    let error =
        validate_password(&"a".repeat(129)).expect_err("oversized password should be rejected");
    assert_eq!(error.code, "validation_failed");
}

#[test]
#[ignore = "run in release mode to calibrate Argon2id on deployment-class hardware"]
fn benchmark_argon2id_latency() {
    let mut samples = Vec::with_capacity(5);
    for _ in 0..5 {
        let started = std::time::Instant::now();
        let hash = hash_password("benchmark passphrase with sufficient entropy")
            .expect("benchmark password should hash");
        assert!(
            verify_password("benchmark passphrase with sufficient entropy", &hash)
                .expect("benchmark password should verify")
        );
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    eprintln!(
        "Argon2id benchmark: median={}ms, m=65536KiB, t=3, p=4",
        median.as_millis()
    );
    assert!(
        median >= std::time::Duration::from_millis(50),
        "Argon2id is hashing too quickly for the current hardware; increase its work factor"
    );
    assert!(
        median <= std::time::Duration::from_secs(2),
        "Argon2id exceeds the authentication latency budget; recalibrate its parameters"
    );
}

#[test]
fn birthdate_rejects_future_date() {
    let tomorrow = NaiveDate::from_ymd_opt(2026, 9, 13).unwrap();
    let err = validate_birthdate_on(tomorrow, today()).expect_err("future date should fail");
    assert_eq!(err.code, "validation_failed");
}

#[test]
fn birthdate_rejects_too_young() {
    let birthday_tomorrow = NaiveDate::from_ymd_opt(2013, 9, 13).unwrap();
    let err = validate_birthdate_on(birthday_tomorrow, today()).expect_err("too young should fail");
    assert_eq!(err.code, "minimum_age");
}

#[test]
fn birthdate_accepts_valid_age() {
    let thirteenth_birthday = NaiveDate::from_ymd_opt(2013, 9, 12).unwrap();
    assert!(validate_birthdate_on(thirteenth_birthday, today()).is_ok());
}

#[test]
fn birthdate_rejects_before_1900() {
    let ancient = NaiveDate::from_ymd_opt(1899, 1, 1).unwrap();
    let err = validate_birthdate_on(ancient, today()).expect_err("pre-1900 should fail");
    assert_eq!(err.code, "validation_failed");
}

#[test]
fn birthdate_accepts_leap_day() {
    let leap_day = NaiveDate::from_ymd_opt(2000, 2, 29).unwrap();
    assert!(validate_birthdate_on(leap_day, today()).is_ok());
}

#[test]
fn region_affects_age_calculation() {
    let instant = Utc.with_ymd_and_hms(2026, 9, 12, 18, 0, 0).unwrap();
    let birthdate = NaiveDate::from_ymd_opt(2013, 9, 13).unwrap();
    assert!(validate_birthdate_on(birthdate, date_in_region_at(None, instant)).is_err());
    assert!(validate_birthdate_on(birthdate, date_in_region_at(Some("NZ"), instant)).is_ok());
}

#[test]
fn region_utc_unknown_falls_back_to_utc() {
    let instant = Utc.with_ymd_and_hms(2026, 9, 12, 23, 30, 0).unwrap();
    assert_eq!(
        date_in_region_at(Some("XX"), instant),
        date_in_region_at(None, instant)
    );
}
