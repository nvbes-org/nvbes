use super::TokenConfig;

#[test]
fn production_requires_https_and_explicit_audiences() {
    let result = TokenConfig::from_values(
        "production",
        "http://identity.nvbes.eu".into(),
        "identity-key-1".into(),
        "private".into(),
        "public".into(),
        "nvbes-account-service".into(),
    );
    assert!(result.is_err());

    let result = TokenConfig::from_values(
        "production",
        "https://identity.nvbes.eu".into(),
        "identity-key-1".into(),
        "private".into(),
        "public".into(),
        " , ".into(),
    );
    assert!(result.is_err());
}

#[test]
fn audiences_are_deduplicated_and_exact() {
    let config = TokenConfig::from_values(
        "test",
        "http://identity.test".into(),
        "identity-key-1".into(),
        "private".into(),
        "public".into(),
        "nvbes-account-service,nvbes-account-service,nvbes-billing-service".into(),
    )
    .unwrap();
    assert_eq!(config.allowed_audiences.len(), 2);
}

#[test]
fn issuer_trailing_slash_is_trimmed_in_test_mode() {
    let config = TokenConfig::from_values(
        "test",
        "http://identity.test/".into(),
        "identity-key-1".into(),
        "private".into(),
        "public".into(),
        "nvbes-account-service".into(),
    )
    .unwrap();
    assert_eq!(config.issuer, "http://identity.test");
}

#[test]
fn key_id_and_audience_identifiers_are_validated() {
    assert!(
        TokenConfig::from_values(
            "test",
            "http://identity.test".into(),
            "ab".into(),
            "private".into(),
            "public".into(),
            "nvbes-account-service".into(),
        )
        .is_err()
    );
    assert!(
        TokenConfig::from_values(
            "test",
            "http://identity.test".into(),
            "identity key".into(),
            "private".into(),
            "public".into(),
            "nvbes-account-service".into(),
        )
        .is_err()
    );
    assert!(
        TokenConfig::from_values(
            "test",
            "http://identity.test".into(),
            "identity-key-1".into(),
            "private".into(),
            "public".into(),
            "bad audience".into(),
        )
        .is_err()
    );
}

#[test]
fn from_env_requires_explicit_key_material() {
    let previous = std::env::var("NVBES_IDENTITY_TOKEN_KEY_ID").ok();
    unsafe {
        std::env::remove_var("NVBES_IDENTITY_TOKEN_KEY_ID");
        std::env::remove_var("NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM");
        std::env::remove_var("NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM");
        std::env::remove_var("NVBES_IDENTITY_TOKEN_AUDIENCES");
    }
    assert!(TokenConfig::from_env("test").is_err());
    if let Some(value) = previous {
        unsafe { std::env::set_var("NVBES_IDENTITY_TOKEN_KEY_ID", value) };
    }
}
