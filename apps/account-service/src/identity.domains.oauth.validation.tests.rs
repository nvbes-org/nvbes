use super::*;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::Digest;

#[test]
fn normalize_scopes_lowercases_sorts_and_deduplicates() {
    let scopes = normalize_scopes(vec![
        " email ".to_string(),
        "openid".to_string(),
        "profile".to_string(),
        "Email".to_string(),
        "".to_string(),
    ]);

    assert_eq!(scopes, vec!["email", "openid", "profile"]);
}

#[test]
fn normalize_resources_trims_sorts_and_deduplicates() {
    let resources = normalize_resources(vec![
        " https://api.example.com ".to_string(),
        "urn:api:1".to_string(),
        "urn:api:1".to_string(),
    ]);

    assert_eq!(resources, vec!["https://api.example.com", "urn:api:1"]);
}

#[test]
fn validate_pkce_for_authorize_requires_challenge_for_every_client() {
    let error =
        validate_pkce_for_authorize("confidential", None, Some("S256")).expect_err("pkce required");

    assert_eq!(error.code, "pkce_required");
}

#[test]
fn validate_pkce_for_exchange_requires_pkce_for_confidential_clients() {
    let error =
        validate_pkce_for_exchange("confidential", None, None, None).expect_err("pkce required");

    assert_eq!(error.code, "pkce_required");
}

#[test]
fn validate_pkce_for_exchange_accepts_s256_verifier() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(verifier.as_bytes()));

    assert!(
        validate_pkce_for_exchange(
            "confidential",
            Some(&challenge),
            Some("S256"),
            Some(verifier)
        )
        .is_ok()
    );
}

#[test]
fn validate_pkce_for_authorize_rejects_plain_method() {
    let error = validate_pkce_for_authorize("public", Some("challenge"), Some("plain"))
        .expect_err("plain PKCE should fail");

    assert_eq!(error.code, "pkce_s256_required");
}

#[test]
fn validate_pkce_for_exchange_requires_explicit_s256_method() {
    let error = validate_pkce_for_exchange("public", Some("challenge"), None, Some("verifier"))
        .expect_err("missing PKCE method should fail");

    assert_eq!(error.code, "pkce_s256_required");
}

#[test]
fn parse_step_up_level_accepts_supported_values() {
    assert_eq!(parse_step_up_level(" AAL2 ").unwrap(), "aal2");
    assert_eq!(parse_client_type(" Service ").unwrap(), "service");
    assert_eq!(
        parse_client_policy_status(" pending_approval ").unwrap(),
        "pending_approval"
    );
}

#[test]
fn parse_step_up_level_rejects_unknown_values() {
    let error = parse_step_up_level("aal4").expect_err("invalid aal should fail");

    assert_eq!(error.code, "validation_failed");
}

#[test]
fn validate_redirect_uri_match_requires_provided_uri() {
    let error =
        validate_redirect_uri_match("https://example.com/callback", None).expect_err("missing uri");

    assert_eq!(error.code, "invalid_request");
}

#[test]
fn validate_redirect_uri_allowed_accepts_registered_uri() {
    let allowed = vec![
        "https://example.com/callback".to_string(),
        "https://example.com/alt".to_string(),
    ];

    assert!(validate_redirect_uri_allowed(&allowed, "https://example.com/alt").is_ok());
}

#[test]
fn validate_redirect_uri_allowed_rejects_unregistered_uri() {
    let allowed = vec!["https://example.com/callback".to_string()];
    let err = validate_redirect_uri_allowed(&allowed, "https://attacker.example/callback")
        .expect_err("expected redirect URI validation to fail");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(err.code, "invalid_redirect_uri");
    assert_eq!(
        err.message,
        "The redirect URI is not registered for this OAuth client."
    );
}

#[test]
fn validate_redirect_uri_match_rejects_mismatch() {
    let err = validate_redirect_uri_match(
        "https://example.com/callback",
        Some("https://example.com/other"),
    )
    .expect_err("expected redirect URI mismatch to fail");

    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(err.code, "invalid_grant");
    assert_eq!(
        err.message,
        "The redirect URI does not match the authorization request."
    );
}
