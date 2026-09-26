use axum::http::StatusCode;

use super::{normalize_email, require_non_empty, slugify, validate_email, validate_password};

#[test]
fn normalize_email_trims_and_lowercases() {
    assert_eq!(normalize_email("  User@Example.COM  "), "user@example.com");
}

#[test]
fn slugify_replaces_non_alphanumeric_runs() {
    assert_eq!(slugify("Hello World!!"), "hello-world");
    assert_eq!(slugify("---already---"), "already");
    assert_eq!(slugify(""), "tenant");
}

#[test]
fn validate_email_rejects_malformed_addresses() {
    for email in ["", "a", "@", "user@", "@example.com"] {
        let error = validate_email(email).expect_err("invalid email");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, "validation_failed");
    }
}

#[test]
fn validate_email_accepts_simple_address() {
    validate_email("user@example.com").expect("valid email");
    validate_email("a@b").expect("minimum length email with @");
}

#[test]
fn validate_email_rejects_length_and_at_boundaries() {
    assert!(validate_email("ab").is_err());
    assert!(validate_email("a@").is_err());
    assert!(validate_email("@b").is_err());
}

#[test]
fn require_non_empty_rejects_whitespace() {
    let error = require_non_empty("Display name", "   ").expect_err("whitespace-only value");
    assert_eq!(error.code, "validation_failed");
}

#[test]
fn require_non_empty_trims_value() {
    let value = require_non_empty("Display name", "  Ada  ").expect("trimmed");
    assert_eq!(value, "Ada");
}

#[test]
fn validate_password_table_covers_length_and_entropy() {
    let cases = [
        ("short", "validation_failed"),
        ("passwordpassword", "weak_password"),
        ("CorrectHorseBatteryStaple123!", "ok"),
    ];
    for (password, expected) in cases {
        let result = validate_password(password);
        match expected {
            "ok" => assert!(result.is_ok()),
            code => {
                let error = result.expect_err("expected failure");
                assert_eq!(error.code, code);
            }
        }
    }

    assert!(validate_password("abcdefg").is_err()); // 7 chars
    assert!(validate_password(&"a".repeat(129)).is_err());
}
