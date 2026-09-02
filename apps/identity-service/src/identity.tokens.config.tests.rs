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
