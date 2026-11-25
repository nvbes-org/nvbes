use uuid::Uuid;

use super::{BillingConfig, BillingPrincipal, OperatorAuth, TokenVerifier};

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

#[test]
fn jwt_verifier_rejects_malformed_and_accepts_scoped_token() {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use openssl::{pkey::PKey, rsa::Rsa};
    use serde::Serialize;

    let private = PKey::from_rsa(Rsa::generate(2048).expect("rsa")).expect("pkey");
    let public_pem = String::from_utf8(private.public_key_to_pem().expect("pub")).expect("utf8");
    let private_pem =
        String::from_utf8(private.private_key_to_pem_pkcs8().expect("priv")).expect("utf8");

    let config = BillingConfig {
        identity_public_key_pem: Some(public_pem),
        ..config_without_key()
    };
    let verifier = TokenVerifier::new(&config).expect("verifier");
    assert!(verifier.verify("not-a-jwt").is_err());

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        scope: Option<String>,
        amr: Option<Vec<String>>,
        exp: u64,
        iat: u64,
    }
    let now = jsonwebtoken::get_current_timestamp();
    let principal_id = Uuid::new_v4();
    let claims = Claims {
        sub: principal_id.to_string(),
        scope: Some("billing:read".into()),
        amr: Some(vec!["totp".into()]),
        exp: now + 300,
        iat: now,
    };
    let token = encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(private_pem.as_bytes()).expect("key"),
    )
    .expect("token");

    let principal = verifier.verify(&token).expect("jwt");
    assert_eq!(principal.id(), principal_id);
    assert!(principal.has_mfa());
    assert!(principal.require_scope("billing:read").is_ok());
    assert!(principal.require_scope("billing:write").is_err());
}

#[test]
fn token_verifier_rejects_invalid_public_key_pem() {
    let config = BillingConfig {
        identity_public_key_pem: Some("not-a-pem".into()),
        ..config_without_key()
    };
    assert!(TokenVerifier::new(&config).is_err());
}

#[test]
fn jwt_verifier_rejects_non_rs256_and_invalid_subject() {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use openssl::{pkey::PKey, rsa::Rsa};
    use serde::Serialize;

    let private = PKey::from_rsa(Rsa::generate(2048).expect("rsa")).expect("pkey");
    let public_pem = String::from_utf8(private.public_key_to_pem().expect("pub")).expect("utf8");
    let private_pem =
        String::from_utf8(private.private_key_to_pem_pkcs8().expect("priv")).expect("utf8");

    let config = BillingConfig {
        identity_public_key_pem: Some(public_pem),
        ..config_without_key()
    };
    let verifier = TokenVerifier::new(&config).expect("verifier");

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        scope: Option<String>,
        amr: Option<Vec<String>>,
        exp: u64,
        iat: u64,
    }
    let now = jsonwebtoken::get_current_timestamp();

    let hs_header = Header::new(Algorithm::HS256);
    let hs_token = encode(
        &hs_header,
        &Claims {
            sub: Uuid::new_v4().to_string(),
            scope: None,
            amr: None,
            exp: now + 300,
            iat: now,
        },
        &EncodingKey::from_secret(b"symmetric-test-secret"),
    )
    .expect("hs token");
    assert!(verifier.verify(&hs_token).is_err());

    let bad_sub = encode(
        &Header::new(Algorithm::RS256),
        &Claims {
            sub: "not-a-uuid".into(),
            scope: None,
            amr: None,
            exp: now + 300,
            iat: now,
        },
        &EncodingKey::from_rsa_pem(private_pem.as_bytes()).expect("key"),
    )
    .expect("bad sub");
    assert!(verifier.verify(&bad_sub).is_err());

    let bare = encode(
        &Header::new(Algorithm::RS256),
        &Claims {
            sub: Uuid::new_v4().to_string(),
            scope: None,
            amr: None,
            exp: now + 300,
            iat: now,
        },
        &EncodingKey::from_rsa_pem(private_pem.as_bytes()).expect("key"),
    )
    .expect("bare claims");
    let principal = verifier.verify(&bare).expect("jwt without scope/amr");
    assert!(!principal.has_mfa());
    assert!(principal.require_scope("billing:read").is_err());
}

#[tokio::test]
async fn principal_extractor_requires_bearer_authorization() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let config = config_without_key();
    let state = crate::app::BillingState {
        db: sqlx::PgPool::connect_lazy("postgres://127.0.0.1:1/unused").expect("lazy"),
        tokens: TokenVerifier::new(&config).expect("verifier"),
        metrics: crate::metrics::install(),
        config,
        email_client: None,
    };

    let mut missing = Request::new(()).into_parts().0;
    let err = BillingPrincipal::from_request_parts(&mut missing, &state)
        .await
        .expect_err("missing auth");
    assert!(matches!(err, crate::error::BillingError::Unauthorized));

    let mut basic = Request::builder()
        .header("authorization", "Basic abc")
        .body(())
        .unwrap()
        .into_parts()
        .0;
    let err = BillingPrincipal::from_request_parts(&mut basic, &state)
        .await
        .expect_err("non-bearer");
    assert!(matches!(err, crate::error::BillingError::Unauthorized));

    let id = Uuid::new_v4();
    let mut ok = Request::builder()
        .header("authorization", format!("Bearer {id}"))
        .body(())
        .unwrap()
        .into_parts()
        .0;
    let principal = BillingPrincipal::from_request_parts(&mut ok, &state)
        .await
        .expect("bearer uuid");
    assert_eq!(principal.id(), id);
}

#[tokio::test]
async fn operator_auth_accepts_configured_token_and_rejects_others() {
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    let config = config_without_key();
    let state = crate::app::BillingState {
        db: sqlx::PgPool::connect_lazy("postgres://127.0.0.1:1/unused").expect("lazy"),
        tokens: TokenVerifier::new(&config).expect("verifier"),
        metrics: crate::metrics::install(),
        config,
        email_client: None,
    };

    let mut missing = Request::new(()).into_parts().0;
    assert!(
        OperatorAuth::from_request_parts(&mut missing, &state)
            .await
            .is_err()
    );

    let mut wrong = Request::builder()
        .header("authorization", "Bearer wrong-token")
        .body(())
        .unwrap()
        .into_parts()
        .0;
    assert!(
        OperatorAuth::from_request_parts(&mut wrong, &state)
            .await
            .is_err()
    );

    let mut ok = Request::builder()
        .header("authorization", "Bearer operator-secret")
        .body(())
        .unwrap()
        .into_parts()
        .0;
    OperatorAuth::from_request_parts(&mut ok, &state)
        .await
        .expect("operator");

    let mut none_cfg = config_without_key();
    none_cfg.operator_token = None;
    let state_none = crate::app::BillingState {
        db: sqlx::PgPool::connect_lazy("postgres://127.0.0.1:1/unused").expect("lazy"),
        tokens: TokenVerifier::new(&none_cfg).expect("verifier"),
        metrics: crate::metrics::install(),
        config: none_cfg,
        email_client: None,
    };
    let mut still = Request::builder()
        .header("authorization", "Bearer operator-secret")
        .body(())
        .unwrap()
        .into_parts()
        .0;
    assert!(
        OperatorAuth::from_request_parts(&mut still, &state_none)
            .await
            .is_err()
    );
}
