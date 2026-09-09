use super::provider_metadata;

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
fn discovery_does_not_claim_unimplemented_userinfo_or_federation() {
    let value = serde_json::to_value(provider_metadata("https://identity.example/")).unwrap();
    assert!(value.get("userinfo_endpoint").is_none());
    assert!(value.get("federation_registration_endpoint").is_none());
}
