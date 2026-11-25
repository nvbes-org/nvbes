use super::validate_password_pair;

#[test]
fn recovery_requires_a_distinct_strong_password() {
    assert!(validate_password_pair("long-password-one", "long-password-two").is_ok());
    assert!(validate_password_pair("same-password", "same-password").is_err());
    assert!(validate_password_pair("short", "long-password-two").is_err());
}
