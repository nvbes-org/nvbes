use super::*;
use jsonwebtoken::{EncodingKey, Header, encode};
use rsa::{
    RsaPrivateKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    rand_core::OsRng,
};
use serde_json::{Value, json};
use std::sync::OnceLock;

fn keys() -> &'static (String, String) {
    static KEYS: OnceLock<(String, String)> = OnceLock::new();
    KEYS.get_or_init(|| {
        let key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
        (
            key.to_pkcs8_pem(Default::default()).unwrap().to_string(),
            key.to_public_key()
                .to_public_key_pem(Default::default())
                .unwrap(),
        )
    })
}

#[tokio::test]
async fn protected_billing_route_requires_live_identity_activity() {
    use axum::{
        Json, Router,
        body::Body,
        http::{Request, StatusCode},
        routing::{get, post},
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use tower::ServiceExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}", listener.local_addr().unwrap());
    let mut c = config();
    c.identity_token_issuer = Some(issuer.clone());
    let mut signed_claims = claims();
    signed_claims["iss"] = json!(issuer);
    let token = sign(&signed_claims, &header());
    let mut response = signed_claims.clone();
    response["active"] = json!(true);
    response["token_type"] = json!("Bearer");
    let response = Arc::new(Mutex::new((StatusCode::OK, response)));
    let requests = Arc::new(AtomicUsize::new(0));
    let identity = Router::new().route(
        "/oauth/introspect",
        post({
            let response = response.clone();
            let requests = requests.clone();
            move || {
                let (status, value) = response.lock().unwrap().clone();
                requests.fetch_add(1, Ordering::SeqCst);
                async move { (status, Json(value)) }
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, identity).await.unwrap();
    });
    let state = crate::app::BillingState {
        db: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1/unused")
            .unwrap(),
        tokens: TokenVerifier::new(&c).unwrap(),
        config: c,
        metrics: crate::metrics::install(),
    };
    let reached = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route(
            "/protected",
            get({
                let reached = reached.clone();
                move |_: crate::auth::BillingPrincipal| {
                    reached.fetch_add(1, Ordering::SeqCst);
                    async { StatusCode::NO_CONTENT }
                }
            }),
        )
        .with_state(state);
    let request = |token: &str| {
        Request::get("/protected")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    };
    assert_eq!(
        app.clone().oneshot(request(&token)).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
    *response.lock().unwrap() = (StatusCode::OK, json!({"active":false}));
    assert_eq!(
        app.clone().oneshot(request(&token)).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    *response.lock().unwrap() = (
        StatusCode::SERVICE_UNAVAILABLE,
        json!({"error":"temporarily_unavailable"}),
    );
    assert_eq!(
        app.clone().oneshot(request(&token)).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        app.oneshot(request("test-arbitrary"))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(requests.load(Ordering::SeqCst), 3);
    assert_eq!(reached.load(Ordering::SeqCst), 1);
    server.abort();
}

fn config() -> BillingConfig {
    BillingConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        database_url: String::new(),
        stripe_secret_key: "sk_test_unused".into(),
        stripe_webhook_secret: "whsec_unused".into(),
        stripe_api_base_url: "https://api.stripe.com".into(),
        app_url: "https://billing.example".into(),
        identity_public_key_pem: Some(keys().1.clone()),
        identity_token_issuer: Some("https://identity.example/".into()),
        identity_token_key_id: Some("identity-test".into()),
        identity_resource_client_id: Some("billing-api".into()),
        identity_resource_secret: Some("A".repeat(43)),
        metrics_token: None,
        operator_token: None,
    }
}
fn claims() -> Value {
    let now = chrono::Utc::now().timestamp() as u64;
    json!({"sub":Uuid::new_v4(), "iss":"https://identity.example/", "aud":AUDIENCE,
        "token_type":"access", "scope":"billing:read billing:checkout", "amr":["pwd","totp"],
        "client_id":"account-web", "exp":now+600,"iat":now,"nbf":now,"auth_time":now-60,
        "step_up_time":now-30,"step_up_expires_at":now+300,
        "jti":Uuid::new_v4(),"sid":Uuid::new_v4(),"grant_id":Uuid::new_v4()})
}
fn header() -> Header {
    let mut h = Header::new(Algorithm::RS256);
    h.typ = Some("at+jwt".into());
    h.kid = Some("identity-test".into());
    h
}
fn sign(claims: &Value, header: &Header) -> String {
    encode(
        header,
        claims,
        &EncodingKey::from_rsa_pem(keys().0.as_bytes()).unwrap(),
    )
    .unwrap()
}

#[test]
fn only_configured_identity_access_tokens_authorize_exact_billing_scopes() {
    let verifier = TokenVerifier::new(&config()).unwrap();
    let claims = claims();
    let principal = verifier.verify(&sign(&claims, &header())).unwrap();
    assert_eq!(principal.id().to_string(), claims["sub"].as_str().unwrap());
    assert!(principal.require_scope("billing:checkout").is_ok());
    assert!(principal.require_scope("billing:write").is_err());
    assert!(principal.require_scope("billing:admin").is_err());
    assert!(principal.has_mfa());
}

#[test]
fn missing_configuration_never_accepts_arbitrary_bearers() {
    let mut c = config();
    c.identity_public_key_pem = None;
    assert!(TokenVerifier::new(&c).is_err());
    c.identity_token_issuer = None;
    c.identity_token_key_id = None;
    c.identity_resource_client_id = None;
    c.identity_resource_secret = None;
    let verifier = TokenVerifier::new(&c).unwrap();
    for token in [
        "anything".into(),
        Uuid::new_v4().to_string(),
        format!("test-{}", Uuid::new_v4()),
        sign(&claims(), &header()),
    ] {
        assert!(verifier.verify(&token).is_err());
    }
}

#[test]
fn valid_signatures_do_not_bypass_claim_or_token_type_boundaries() {
    let verifier = TokenVerifier::new(&config()).unwrap();
    let base = claims();
    let now = base["iat"].as_u64().unwrap();
    for (key, value) in [
        ("iss", json!("https://identity.example")),
        ("aud", json!("nvbes-account-service")),
        ("aud", json!([AUDIENCE, "nvbes-account-service"])),
        ("token_type", json!("id")),
        ("exp", json!(now - 1)),
        ("exp", json!(now + 901)),
        ("iat", json!(now + 100)),
        ("nbf", json!(now + 100)),
        ("auth_time", json!(now + 100)),
        ("scope", json!("billing:admin")),
        ("scope", json!("account:read")),
        ("scope", json!("billing:write")),
        ("amr", json!([])),
        ("amr", json!(["unknown"])),
        ("client_id", json!("")),
        ("grant_id", json!("not-a-uuid")),
        ("sid", json!(Uuid::nil())),
        ("cnf", json!({"jkt":"bound-token-requires-DPoP"})),
    ] {
        let mut changed = base.clone();
        changed[key] = value;
        assert!(
            verifier.verify(&sign(&changed, &header())).is_err(),
            "accepted {key}: {changed}"
        );
    }
    for key in [
        "exp",
        "nbf",
        "iss",
        "aud",
        "client_id",
        "sid",
        "grant_id",
        "auth_time",
        "token_type",
    ] {
        let mut changed = base.clone();
        changed.as_object_mut().unwrap().remove(key);
        assert!(
            verifier.verify(&sign(&changed, &header())).is_err(),
            "missing {key}"
        );
    }
    for key in ["typ", "kid", "jku"] {
        let mut h = header();
        match key {
            "typ" => h.typ = Some("JWT".into()),
            "kid" => h.kid = Some("other".into()),
            _ => h.jku = Some("https://evil.example/jwks".into()),
        }
        assert!(verifier.verify(&sign(&base, &h)).is_err());
    }
    let mut token = sign(&base, &header()).into_bytes();
    let index = token.len() - 40;
    token[index] = if token[index] == b'A' { b'B' } else { b'A' };
    assert!(
        verifier
            .verify(std::str::from_utf8(&token).unwrap())
            .is_err()
    );
}

#[test]
fn expired_step_up_does_not_report_fresh_strong_authentication() {
    let verifier = TokenVerifier::new(&config()).unwrap();
    let mut c = claims();
    let now = c["iat"].as_u64().unwrap();
    c["iat"] = json!(now - 120);
    c["nbf"] = json!(now - 120);
    c["auth_time"] = json!(now - 200);
    c["step_up_time"] = json!(now - 150);
    c["step_up_expires_at"] = json!(now - 1);
    assert!(!verifier.verify(&sign(&c, &header())).unwrap().has_mfa());
    c["amr"] = json!(["webauthn"]);
    c.as_object_mut().unwrap().remove("step_up_time");
    c.as_object_mut().unwrap().remove("step_up_expires_at");
    assert!(verifier.verify(&sign(&c, &header())).unwrap().has_mfa());
    c["auth_time"] = json!(now - 301);
    assert!(!verifier.verify(&sign(&c, &header())).unwrap().has_mfa());
}

#[test]
fn duplicate_or_ambiguous_authorization_headers_are_rejected() {
    use axum::{body::Body, http::Request};
    for values in [
        vec!["Bearer one", "Bearer two"],
        vec!["Bearer "],
        vec!["Bearer one two"],
        vec!["DPoP one"],
    ] {
        let mut request = Request::builder();
        for value in values {
            request = request.header("authorization", value);
        }
        let (parts, _) = request.body(Body::empty()).unwrap().into_parts();
        assert!(super::super::bearer(&parts).is_err());
    }
}
