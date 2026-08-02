use super::{DEVELOPMENT_DATA_KEY, DEVELOPMENT_HMAC_KEY, csv_set, key, local_smtp};

#[test]
fn cryptographic_keys_must_decode_to_exactly_32_bytes() {
    assert!(key("TEST_KEY", Some(DEVELOPMENT_DATA_KEY)).is_ok());
    assert!(key("TEST_KEY", Some(DEVELOPMENT_HMAC_KEY)).is_ok());
    assert!(key("TEST_KEY", Some("c2hvcnQ=")).is_err());
}

#[test]
fn producer_allowlist_rejects_an_empty_value() {
    assert!(csv_set(" , ").is_err());
    assert_eq!(csv_set("identity-service,billing-worker").unwrap().len(), 2);
}

#[test]
fn smtp_is_local_only_and_supports_an_unauthenticated_mail_sink() {
    assert!(
        local_smtp(
            "development",
            "127.0.0.1".to_string(),
            1025,
            None,
            None,
            false,
        )
        .is_ok()
    );
    assert!(
        local_smtp(
            "production",
            "127.0.0.1".to_string(),
            1025,
            None,
            None,
            false,
        )
        .is_err()
    );
}
