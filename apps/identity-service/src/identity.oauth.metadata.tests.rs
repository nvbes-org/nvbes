use super::provider_metadata;

#[test]
fn endpoints_have_one_slash_without_rewriting_the_issuer_claim() {
    for issuer in ["https://identity.example", "https://identity.example/"] {
        let metadata = provider_metadata(issuer);
        assert_eq!(metadata.issuer, issuer);
        assert_eq!(
            metadata.authorization_endpoint,
            "https://identity.example/oauth/authorize"
        );
        assert_eq!(
            metadata.token_endpoint,
            "https://identity.example/oauth/token"
        );
        assert_eq!(
            metadata.userinfo_endpoint,
            "https://identity.example/oauth/userinfo"
        );
        assert_eq!(metadata.jwks_uri, "https://identity.example/oauth/jwks");
    }
}

#[test]
fn discovery_uses_the_exact_issuer_and_only_mounted_capabilities() {
    let metadata = provider_metadata("https://identity.example/");
    assert_eq!(metadata.issuer, "https://identity.example/");
    assert_eq!(
        metadata.authorization_endpoint,
        "https://identity.example/oauth/authorize"
    );
    assert_eq!(
        metadata.token_endpoint,
        "https://identity.example/oauth/token"
    );
    assert_eq!(metadata.jwks_uri, "https://identity.example/oauth/jwks");
    assert_eq!(metadata.response_types_supported, ["code"]);
    assert_eq!(metadata.code_challenge_methods_supported, ["S256"]);
    assert_eq!(metadata.subject_types_supported, ["public"]);
}

#[test]
fn discovery_claims_userinfo_but_not_federation() {
    let value = serde_json::to_value(provider_metadata("https://identity.example/")).unwrap();
    assert_eq!(
        value.get("userinfo_endpoint").and_then(|v| v.as_str()),
        Some("https://identity.example/oauth/userinfo")
    );
    assert!(value.get("federation_registration_endpoint").is_none());
}
