use uuid::Uuid;

use super::{BillingConfig, BillingPrincipal, TokenVerifier};

fn config_without_key() -> BillingConfig {
    BillingConfig {
        bind_addr: "127.0.0.1:8080".parse().unwrap(),
        database_url: "postgres://localhost/test".into(),
        stripe_secret_key: "sk_test".into(),
        stripe_webhook_secret: "whsec".into(),
        stripe_api_base_url: "https://api.stripe.com".into(),
        identity_public_key_pem: None,
        metrics_token: None,
        operator_token: Some("operator-secret".into()),
        app_url: "https://nvbes.test".into(),
        email_grpc_endpoint: None,
        email_token: None,
    }
}

#[test]
fn principal_scopes_and_mfa_verification() {
    let principal = BillingPrincipal {
        id: Uuid::new_v4(),
        scopes: vec!["billing:write".into()],
        amr: vec!["totp".into()],
    };
    assert_eq!(principal.id(), principal.id);
    assert!(principal.has_mfa());
    assert!(principal.require_scope("billing:write").is_ok());
    assert!(principal.require_scope("billing:admin").is_err());
}

#[test]
fn admin_scope_grants_any_billing_scope() {
    let principal = BillingPrincipal {
        id: Uuid::nil(),
        scopes: vec!["billing:admin".into()],
        amr: vec!["pwd".into()],
    };
    assert!(!principal.has_mfa());
    assert!(principal.require_scope("billing:read").is_ok());
    assert!(principal.require_scope("billing:write").is_ok());
}

#[test]
fn webauthn_counts_as_mfa() {
    let principal = BillingPrincipal {
        id: Uuid::nil(),
        scopes: vec![],
        amr: vec!["webauthn".into()],
    };
    assert!(principal.has_mfa());
}

#[test]
fn development_verifier_accepts_uuid_and_test_prefix() {
    let verifier = TokenVerifier::new(&config_without_key()).expect("verifier");
    let id = Uuid::new_v4();
    let principal = verifier.verify(&id.to_string()).expect("uuid token");
    assert_eq!(principal.id(), id);
    assert!(principal.require_scope("billing:read").is_ok());

    let prefixed = verifier
        .verify(&format!("test-{id}"))
        .expect("prefixed token");
    assert_eq!(prefixed.id(), id);

    let fallback = verifier.verify("not-a-uuid").expect("nil fallback");
    assert_eq!(fallback.id(), Uuid::nil());
}
