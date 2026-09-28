use super::{hash_token, normalize_email, random_token, validate_password, validate_password_pair};

#[test]
fn email_is_normalized_without_leaking_into_a_token() {
    assert_eq!(
        normalize_email(" Person@Example.COM ").unwrap(),
        "person@example.com"
    );
    assert!(normalize_email("invalid").is_err());
    assert!(normalize_email("@example.com").is_err());
    assert!(normalize_email("user@localhost").is_err());
    assert!(normalize_email(&format!("{}@x.com", "a".repeat(320))).is_err());
}

#[test]
fn opaque_tokens_have_fixed_entropy_and_are_hashed_at_rest() {
    let first = random_token();
    let second = random_token();
    assert_eq!(first.len(), 43);
    assert_ne!(first, second);
    assert_eq!(hash_token(&first).len(), 32);
    assert_ne!(hash_token(&first), first.as_bytes());
}

#[test]
fn recovery_requires_a_distinct_strong_password() {
    assert!(validate_password_pair("long-password-one", "long-password-two").is_ok());
    assert!(validate_password_pair("same-password", "same-password").is_err());
    assert!(validate_password_pair("short", "long-password-two").is_err());
    assert!(validate_password("long-password-one").is_ok());
    assert!(validate_password("short").is_err());
    assert!(validate_password(&"x".repeat(1025)).is_err());
}
