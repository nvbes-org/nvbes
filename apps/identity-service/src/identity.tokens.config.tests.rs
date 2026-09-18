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
        "http://localhost:3000".into(),
        "identity-key-1".into(),
        "private".into(),
        "public".into(),
        "nvbes-account-service,nvbes-account-service,nvbes-billing-service".into(),
    )
    .unwrap();
    assert_eq!(config.allowed_audiences.len(), 2);
}

#[test]
fn userinfo_audience_is_configurable_without_accepting_unknown_resources() {
    for (audience, accepted) in [
        ("nvbes-identity-userinfo", true),
        ("nvbes-identity-userinfo-extra", false),
    ] {
        let result = TokenConfig::from_values(
            "production",
            "https://identity.example".into(),
            "identity-key-1".into(),
            "private".into(),
            "public".into(),
            audience.into(),
        );
        assert_eq!(result.is_ok(), accepted);
    }
}

#[test]
fn issuer_rejects_credentials_queries_paths_and_nonlocal_http() {
    for issuer in [
        "https://user:password@identity.example",
        "https://identity.example/?x=1",
        "https://identity.example/#fragment",
        "https://identity.example/path",
        "http://identity.example",
        "ftp://localhost",
        "https://identity.example\\attacker",
    ] {
        assert!(
            TokenConfig::from_values(
                "development",
                issuer.into(),
                "key-1".into(),
                "private".into(),
                "public".into(),
                "nvbes-account-service".into()
            )
            .is_err()
        );
    }
    let exact = TokenConfig::from_values(
        "production",
        "https://identity.example/".into(),
        "key-1".into(),
        "private".into(),
        "public".into(),
        "nvbes-account-service".into(),
    )
    .unwrap();
    assert_eq!(exact.issuer, "https://identity.example/");
    assert!(
        exact
            .with_verification_keys(
                r#"[{"kid":"key-1","public_key_pem":"public","accept_until":10}]"#
            )
            .is_err()
    );
}
