use super::{
    normalize_domain, normalize_enterprise_provider_family, normalize_federated_provider_type,
    normalize_registry_status, opt_trimmed, validate_federation_endpoint_url,
    validate_federation_endpoint_url_allowed,
};

#[test]
fn normalize_domain_trims_and_lowercases() {
    assert_eq!(
        normalize_domain(" IdP.Example.COM. ").expect("domain should normalize"),
        "idp.example.com"
    );
}

#[test]
fn normalize_federated_provider_type_accepts_supported_values() {
    assert_eq!(
        normalize_federated_provider_type(" OIDC ").expect("provider type should normalize"),
        "oidc"
    );
}

#[test]
fn normalize_enterprise_provider_family_accepts_supported_enterprise_idps() {
    assert_eq!(
        normalize_enterprise_provider_family(Some(" Google-Workspace "))
            .expect("provider family should normalize"),
        "google_workspace"
    );
    assert_eq!(
        normalize_enterprise_provider_family(Some("azure_ad"))
            .expect("provider family should normalize"),
        "azure_ad"
    );
    assert_eq!(
        normalize_enterprise_provider_family(Some("okta"))
            .expect("provider family should normalize"),
        "okta"
    );
}

#[test]
fn normalize_enterprise_provider_family_rejects_unknown_values() {
    let err = normalize_enterprise_provider_family(Some("github"))
        .expect_err("unsupported provider family should fail");

    assert_eq!(err.code, "validation_failed");
}

#[test]
fn normalize_registry_status_defaults_to_active_and_trims() {
    assert_eq!(
        normalize_registry_status(None).expect("default status should be active"),
        "active"
    );
    assert_eq!(
        normalize_registry_status(Some(" Pending ")).expect("status should normalize"),
        "pending"
    );
}

#[test]
fn opt_trimmed_discards_blank_values() {
    assert_eq!(opt_trimmed(Some("  ".to_string())), None);
    assert_eq!(
        opt_trimmed(Some(" https://metadata.example ".to_string())),
        Some("https://metadata.example".to_string())
    );
}

#[test]
fn validate_federation_endpoint_url_rejects_credentials() {
    let err = validate_federation_endpoint_url("https://user:pass@example.com")
        .expect_err("expected credentials to be rejected");
    assert_eq!(err.code, "validation_failed");
}

#[test]
fn validate_federation_endpoint_url_rejects_missing_host() {
    let err = validate_federation_endpoint_url("https://")
        .expect_err("expected missing host to be rejected");

    assert_eq!(err.code, "validation_failed");
}

#[tokio::test]
async fn validate_federation_endpoint_url_allows_public_https_in_strict_mode() {
    let url = validate_federation_endpoint_url_allowed("https://1.1.1.1/metadata", true)
        .await
        .expect("expected public HTTPS URL to be accepted");
    assert_eq!(url, "https://1.1.1.1/metadata");
}

#[tokio::test]
async fn validate_federation_endpoint_url_rejects_loopback_in_strict_mode() {
    let err = validate_federation_endpoint_url_allowed("https://localhost/metadata", true)
        .await
        .expect_err("expected loopback URL to be rejected");
    assert_eq!(err.code, "validation_failed");
}
