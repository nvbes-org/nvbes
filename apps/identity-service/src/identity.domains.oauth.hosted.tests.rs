use super::hosted_keys::{hosted_authorization_state_key, hosted_authorization_state_ttl_seconds};
use super::hosted_service::{
    build_hosted_client_display, build_hosted_login_url, build_oauth_error_redirect_url,
    build_oauth_redirect_url,
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
fn hosted_routes_expose_router() {
    let _router = super::hosted_routes::router();
}

#[test]
fn hosted_client_display_prefers_developer_consent_branding() {
    let display = build_hosted_client_display(
        "client_123".to_string(),
        "Fallback Client".to_string(),
        crate::developer_client::DeveloperConsentScreen {
            product_name: "Branded App".to_string(),
            logo_url: Some("https://cdn.example/logo.png".to_string()),
            support_url: Some("https://example.test/support".to_string()),
            privacy_url: Some("https://example.test/privacy".to_string()),
            terms_url: Some("https://example.test/terms".to_string()),
            description: "Use Branded App with nvbes.".to_string(),
            brand_color: Some("#123456".to_string()),
            custom_css: None,
            help_text: Some("Contact support for access.".to_string()),
        },
    );

    assert_eq!(display.client_id, "client_123");
    assert_eq!(display.name, "Branded App");
    assert_eq!(
        display.logo_url.as_deref(),
        Some("https://cdn.example/logo.png")
    );
    assert_eq!(
        display.description.as_deref(),
        Some("Use Branded App with nvbes.")
    );
}
