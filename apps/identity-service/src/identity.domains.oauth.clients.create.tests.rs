use super::{require_workspace_scope, validate_event_endpoint, validate_redirect_uris};
use uuid::Uuid;

#[test]
fn event_endpoints_accept_absent_and_public_https_urls() {
    assert!(validate_event_endpoint(None, "receiver").is_ok());
    assert!(validate_event_endpoint(Some("https://events.example.test/oauth"), "receiver").is_ok());
}

#[test]
fn event_endpoints_reject_ambiguous_or_private_targets() {
    for endpoint in [
        "not-a-url",
        "http://events.example.test/oauth",
        "https://user:secret@events.example.test/oauth",
        "https://events.example.test/oauth#fragment",
        "https://127.0.0.1/oauth",
        "https://10.0.0.1/oauth",
        "https://169.254.1.1/oauth",
        "https://0.0.0.0/oauth",
        "https://[::1]/oauth",
        "https://[::]/oauth",
    ] {
        assert!(
            validate_event_endpoint(Some(endpoint), "receiver").is_err(),
            "{endpoint} must be rejected"
        );
    }
}

#[test]
fn redirect_uris_accept_exact_https_and_native_loopback_urls() {
    assert!(
        validate_redirect_uris(
            &["https://client.example.test/callback".to_string()],
            "confidential"
        )
        .is_ok()
    );
    for redirect_uri in [
        "http://127.0.0.1:49152/callback",
        "http://localhost:49152/callback",
        "http://[::1]:49152/callback",
    ] {
        assert!(validate_redirect_uris(&[redirect_uri.to_string()], "public").is_ok());
    }
}

#[test]
fn redirect_uris_reject_non_exact_or_unsafe_urls() {
    for redirect_uris in [
        vec!["not-a-url".to_string()],
        vec!["http://client.example.test/callback".to_string()],
        vec!["http://localhost/callback".to_string()],
        vec!["https://user:secret@client.example.test/callback".to_string()],
        vec!["https://client.example.test/callback#fragment".to_string()],
        vec!["https://*.example.test/callback".to_string()],
        vec![
            "https://client.example.test/callback".to_string(),
            "https://client.example.test/callback".to_string(),
        ],
    ] {
        assert!(validate_redirect_uris(&redirect_uris, "confidential").is_err());
    }
}

#[test]
fn redirect_uri_registration_is_bounded() {
    let redirect_uris = (0..21)
        .map(|index| format!("https://client.example.test/callback/{index}"))
        .collect::<Vec<_>>();
    assert!(validate_redirect_uris(&redirect_uris, "confidential").is_err());
}

#[test]
fn service_clients_require_workspace_ownership_scope() {
    let workspace_id = Uuid::new_v4();
    assert_eq!(
        require_workspace_scope("workspace", workspace_id).expect("workspace scope"),
        workspace_id
    );
    assert!(require_workspace_scope("tenant", workspace_id).is_err());
}
