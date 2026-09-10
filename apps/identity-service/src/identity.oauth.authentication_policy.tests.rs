use super::*;

#[test]
fn unknown_policy_is_rejected_and_browser_input_cannot_lower_registration() {
    let config = r#"[{"client_id":"account-web","display_name":"Account","redirect_uris":["https://account.example/callback"],"post_logout_redirect_uris":[],"resources":{"https://api.example/account":{"audience":"nvbes-account-service","scopes":["account:read"]}},"allow_refresh":false,"require_dpop":false,"minimum_authentication":"recent_webauthn"}]"#;
    let clients = crate::oauth::clients::ClientRegistry::from_json(config, false).unwrap();
    assert!(
        crate::oauth::clients::ClientRegistry::from_json(
            &config.replace("recent_webauthn", "anything"),
            false
        )
        .is_err()
    );
    let input: crate::oauth::request::AuthorizationInput = serde_json::from_value(serde_json::json!({
        "client_id":"account-web","redirect_uri":"https://account.example/callback",
        "response_type":"code","scope":"openid account:read","resource":"https://api.example/account",
        "state":"random-state-for-test","nonce":"random-nonce-for-test",
        "code_challenge":"E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM","code_challenge_method":"S256",
        "minimum_authentication":"primary"
    })).unwrap();
    assert!(
        input.validate(&clients).unwrap().minimum_authentication
            == AuthenticationPolicy::RecentWebauthn
    );
}

#[test]
fn strong_policy_requires_real_recent_evidence_and_preserves_its_expiry() {
    let now = Utc::now();
    let mut auth = Authentication {
        authenticated_at: now - Duration::minutes(20),
        primary_amr: "pwd".into(),
        step_up_method: None,
        step_up_at: None,
        step_up_expires_at: None,
    };
    assert_eq!(
        AuthenticationPolicy::Primary.deadline(&auth, now).unwrap(),
        None
    );
    for policy in [
        AuthenticationPolicy::RecentMfa,
        AuthenticationPolicy::RecentWebauthn,
    ] {
        assert!(policy.deadline(&auth, now).is_err());
    }
    auth.step_up_method = Some("totp".into());
    auth.step_up_at = Some(now - Duration::minutes(4));
    auth.step_up_expires_at = Some(now + Duration::minutes(6));
    assert_eq!(
        AuthenticationPolicy::RecentMfa
            .deadline(&auth, now)
            .unwrap(),
        Some(now + Duration::minutes(1))
    );
    assert!(
        AuthenticationPolicy::RecentWebauthn
            .deadline(&auth, now)
            .is_err()
    );
    auth.step_up_method = Some("webauthn".into());
    auth.step_up_expires_at = Some(now + Duration::seconds(10));
    assert_eq!(
        AuthenticationPolicy::RecentWebauthn
            .deadline(&auth, now)
            .unwrap(),
        Some(now + Duration::seconds(10))
    );
    assert!(
        AuthenticationPolicy::RecentWebauthn
            .deadline(&auth, now + Duration::seconds(10))
            .is_err()
    );
    auth.step_up_at = Some(now + Duration::seconds(1));
    assert!(
        AuthenticationPolicy::RecentMfa
            .deadline(&auth, now)
            .is_err()
    );
    auth.step_up_at = None;
    auth.step_up_method = None;
    auth.step_up_expires_at = None;
    auth.primary_amr = "webauthn".into();
    auth.authenticated_at = now - Duration::minutes(1);
    assert_eq!(
        AuthenticationPolicy::RecentWebauthn
            .deadline(&auth, now)
            .unwrap(),
        Some(now + Duration::minutes(4))
    );
}
