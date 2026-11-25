use serde_json::{Value, json};

use super::{clients::ClientRegistry, error::OAuthError, pkce, request::AuthorizationInput};

const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

fn registration() -> Value {
    json!([{
        "client_id": "account-web", "display_name": "Account",
        "redirect_uris": ["https://account.example/callback"],
        "post_logout_redirect_uris": ["https://account.example/"],
        "resources": {"https://api.example/account": {"audience":"nvbes-account-service", "scopes":["account:read", "account:write"]}},
        "allow_refresh": true, "require_dpop": false
    }])
}

fn input() -> AuthorizationInput {
    AuthorizationInput {
        client_id: "account-web".into(),
        redirect_uri: "https://account.example/callback".into(),
        response_type: "code".into(),
        resource: "https://api.example/account".into(),
        scope: "openid account:read".into(),
        state: "state-with-enough-entropy".into(),
        nonce: "nonce-with-enough-entropy".into(),
        code_challenge: CHALLENGE.into(),
        code_challenge_method: "S256".into(),
        dpop_jkt: None,
        max_age: None,
        prompt: None,
    }
}

#[test]
fn pkce_matches_rfc7636_vector_and_refuses_downgrade() {
    assert!(pkce::validate_challenge(CHALLENGE, "S256").is_ok());
    assert!(pkce::verify(CHALLENGE, VERIFIER).is_ok());
    assert_eq!(
        pkce::verify(CHALLENGE, &"a".repeat(43)),
        Err(OAuthError::InvalidGrant)
    );
    for method in ["plain", "s256", "", "S256 "] {
        assert!(pkce::validate_challenge(CHALLENGE, method).is_err());
    }
    for verifier in ["x".repeat(42), "x".repeat(129), "!".repeat(43)] {
        assert!(pkce::verify(CHALLENGE, &verifier).is_err());
    }
}

#[test]
fn registry_refuses_unsafe_urls_unknown_fields_and_duplicate_clients() {
    for uri in [
        "https://*.example/callback",
        "https://user:pass@example/cb",
        "https://account.example/cb#x",
        "http://account.example/cb",
        "javascript:alert(1)",
    ] {
        let mut config = registration();
        config[0]["redirect_uris"] = json!([uri]);
        assert!(
            ClientRegistry::from_json(&config.to_string(), false).is_err(),
            "{uri}"
        );
    }
    let mut config = registration();
    config[0]["client_secret"] = json!("never-in-browser");
    assert!(ClientRegistry::from_json(&config.to_string(), false).is_err());
    let duplicate = json!([registration()[0], registration()[0]]);
    assert!(ClientRegistry::from_json(&duplicate.to_string(), false).is_err());
}

#[test]
fn loopback_http_is_explicitly_development_only() {
    for uri in [
        "http://127.0.0.1:3001/cb",
        "http://localhost:3001/cb",
        "http://[::1]:3001/cb",
    ] {
        let mut config = registration();
        config[0]["redirect_uris"] = json!([uri]);
        assert!(ClientRegistry::from_json(&config.to_string(), true).is_ok());
        assert!(ClientRegistry::from_json(&config.to_string(), false).is_err());
    }
}

#[test]
fn authorization_rejects_redirect_confusion_and_foreign_scopes() {
    let registry = ClientRegistry::from_json(&registration().to_string(), false).unwrap();
    let validated = input().validate(&registry).unwrap();
    assert_eq!(validated.scope(), "account:read openid");
    assert_eq!(validated.audience(), "nvbes-account-service");
    for redirect in [
        "https://account.example/callback/",
        "https://account.example/callback?next=evil",
        "https://account.example.evil/callback",
    ] {
        let mut request = input();
        request.redirect_uri = redirect.into();
        assert!(matches!(
            request.validate(&registry),
            Err(OAuthError::InvalidClient)
        ));
    }
    for scope in [
        "openid billing:read",
        "openid account:close",
        "openid",
        "openid account:read account:read",
        "openid  account:read",
        "openid\taccount:read",
    ] {
        let mut request = input();
        request.scope = scope.into();
        assert!(
            matches!(request.validate(&registry), Err(OAuthError::InvalidScope)),
            "{scope}"
        );
    }
    let mut request = input();
    request.resource = "https://api.example/billing".into();
    assert!(matches!(
        request.validate(&registry),
        Err(OAuthError::InvalidTarget)
    ));
}

#[test]
fn request_cannot_disable_registered_dpop_or_enable_refresh() {
    let mut config = registration();
    config[0]["require_dpop"] = json!(true);
    config[0]["allow_refresh"] = json!(false);
    let registry = ClientRegistry::from_json(&config.to_string(), false).unwrap();
    assert!(matches!(
        input().validate(&registry),
        Err(OAuthError::InvalidRequest)
    ));
    let mut request = input();
    request.dpop_jkt = Some(CHALLENGE.into());
    assert!(request.validate(&registry).is_ok());
    let mut request = input();
    request.dpop_jkt = Some(CHALLENGE.into());
    request.scope.push_str(" offline_access");
    assert!(matches!(
        request.validate(&registry),
        Err(OAuthError::InvalidScope)
    ));
}

#[test]
fn persisted_requests_are_invalid_after_a_registration_is_narrowed() {
    let config = registration();
    let registry = ClientRegistry::from_json(&config.to_string(), false).unwrap();
    let request = input().validate(&registry).unwrap();
    let stored = serde_json::to_value(request).unwrap();
    let mut narrower = config;
    narrower[0]["resources"]["https://api.example/account"]["scopes"] = json!(["account:write"]);
    let registry = ClientRegistry::from_json(&narrower.to_string(), false).unwrap();
    assert!(super::request::AuthorizationRequest::restore(stored, &registry).is_err());
}
