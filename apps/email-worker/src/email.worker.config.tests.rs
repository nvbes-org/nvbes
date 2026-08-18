use super::{
    DEVELOPMENT_DATA_KEY, DEVELOPMENT_HMAC_KEY, key, local_smtp, producers,
    validate_shared_bind_address,
};

#[test]
fn cryptographic_keys_must_decode_to_exactly_32_bytes() {
    assert!(key("TEST_KEY", Some(DEVELOPMENT_DATA_KEY)).is_ok());
    assert!(key("TEST_KEY", Some(DEVELOPMENT_HMAC_KEY)).is_ok());
    assert!(key("TEST_KEY", Some("c2hvcnQ=")).is_err());
}

#[test]
fn producer_credentials_are_bound_and_unique_in_production() {
    assert!(producers::parse(" , ", "production").is_err());
    assert!(
        producers::parse(
            "identity-service=01234567890123456789012345678901,billing-worker=01234567890123456789012345678901",
            "production",
        )
        .is_err()
    );
    assert_eq!(
        producers::parse(
            "identity-service=01234567890123456789012345678901,billing-worker=abcdefghijklmnopqrstuvwxyzABCDEF",
            "production",
        )
        .unwrap()
        .len(),
        2
    );
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

#[test]
fn http_and_grpc_share_the_serverless_listener() {
    let shared = "127.0.0.1:3040".parse().unwrap();
    let separate = "127.0.0.1:3041".parse().unwrap();

    assert!(validate_shared_bind_address(shared, shared).is_ok());
    assert!(validate_shared_bind_address(shared, separate).is_err());
}
