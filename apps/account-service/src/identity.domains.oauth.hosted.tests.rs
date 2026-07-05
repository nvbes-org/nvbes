use super::hosted_keys::{hosted_authorization_state_key, hosted_authorization_state_ttl_seconds};
use super::hosted_routes::HostedStartRequest;
use super::hosted_service::{
    build_hosted_login_url, build_oauth_error_redirect_url, build_oauth_redirect_url,
};

#[test]
fn hosted_authorization_state_key_is_namespaced() {
    let key = hosted_authorization_state_key("state_123");
    assert_eq!(key, "nvbes:identity:oauth:hosted-login:state_123");
}

#[test]
fn hosted_authorization_state_ttl_is_short_lived() {
    assert_eq!(hosted_authorization_state_ttl_seconds(), 300);
}

#[test]
fn hosted_login_url_contains_state_reference() {
    let url = build_hosted_login_url("https://identity.example", "hosted_abc");

    assert_eq!(url, "https://identity.example/login?state_id=hosted_abc");
}

#[test]
fn oauth_redirect_url_preserves_state() {
    let url = build_oauth_redirect_url(
        "https://app.example/callback",
        &[("code", "auth_code_1"), ("state", "opaque_state")],
    )
    .expect("redirect should build");

    assert_eq!(
        url,
        "https://app.example/callback?code=auth_code_1&state=opaque_state"
    );
}

#[test]
fn oauth_error_redirect_includes_error_and_state() {
    let url = build_oauth_error_redirect_url(
        "https://app.example/callback",
        "access_denied",
        "The user denied access.",
        Some("state_1"),
    )
    .expect("error redirect should build");

    assert_eq!(
        url,
        "https://app.example/callback?error=access_denied&error_description=The+user+denied+access.&state=state_1"
    );
}

#[test]
fn hosted_start_request_accepts_authorize_parameters() {
    let json = serde_json::json!({
        "client_id": "drive_web",
        "redirect_uri": "https://drive.example/callback",
        "scope": "openid profile email",
        "state": "state_1",
        "code_challenge": "challenge",
        "code_challenge_method": "S256"
    });

    let request: HostedStartRequest = serde_json::from_value(json).expect("request should parse");

    assert_eq!(request.client_id, "drive_web");
    assert_eq!(request.redirect_uri, "https://drive.example/callback");
    assert_eq!(request.code_challenge_method.as_deref(), Some("S256"));
}

#[test]
fn hosted_routes_expose_router() {
    let _router = super::hosted_routes::router();
}
