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
fn config() -> AccountConfig {
    AccountConfig {
        environment: "test".into(),
        database_url: "postgres://unused:unused@127.0.0.1/unused".into(),
        database_max_connections: 1,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        token_issuer: "https://identity.example/".into(),
        token_audience: "nvbes-account-service".into(),
        token_key_id: "identity-test".into(),
        token_public_key_pem: keys().1.clone(),
        identity_resource_client_id: "account-api".into(),
        identity_resource_secret: "A".repeat(43),
        metrics_token: "test-metrics-token-not-used".into(),
        sentry_dsn: None,
        sentry_traces_sample_rate: 0.0,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    }
}
fn claims() -> Value {
    let now = chrono::Utc::now().timestamp() as u64;
    json!({"sub":Uuid::new_v4(),"sid":Uuid::new_v4(),"grant_id":Uuid::new_v4(),"jti":Uuid::new_v4(),"iss":"https://identity.example/","aud":"nvbes-account-service","client_id":"account-web","token_type":"access","scope":"account:read account:write account:export account:close","amr":["pwd","totp"],"auth_time":now-60,"iat":now,"nbf":now,"exp":now+600,"step_up_time":now-30,"step_up_expires_at":now+300})
}
fn sign(value: &Value) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.typ = Some("at+jwt".into());
    header.kid = Some("identity-test".into());
    encode(
        &header,
        value,
        &EncodingKey::from_rsa_pem(keys().0.as_bytes()).unwrap(),
    )
    .unwrap()
}

#[test]
fn verified_claims_preserve_the_exact_identity_contract() {
    let verifier = TokenVerifier::new(&config()).unwrap();
    let claims = claims();
    let (principal, expected) = verifier.validated(&sign(&claims)).unwrap();
    assert_eq!(principal.id.to_string(), claims["sub"].as_str().unwrap());
    assert_eq!(expected.iss, "https://identity.example/");
    assert_eq!(expected.token_type, "Bearer");
    assert!(principal.require("account:close").is_ok());
    assert!(principal.require("billing:read").is_err());
    assert!(principal.require_step_up().is_ok());
    let now = claims["iat"].as_u64().unwrap();
    for (field, value) in [
        ("iss", json!("https://identity.example")),
        ("aud", json!("nvbes-billing-service")),
        ("token_type", json!("id")),
        ("scope", json!("billing:read")),
        ("scope", json!("account:read account:read")),
        ("amr", json!(["totp"])),
        ("amr", json!(["pwd", "pwd"])),
        ("cnf", json!({"jkt":"DPoP-must-not-be-Bearer"})),
        ("iat", json!(now + 30)),
        ("nbf", json!(now + 30)),
        ("exp", json!(now - 1)),
        ("exp", json!(now + 901)),
        ("grant_id", json!("bad-id")),
        ("auth_time", json!(now + 30)),
    ] {
        let mut value_claims = claims.clone();
        value_claims[field] = value;
        assert!(
            verifier.validated(&sign(&value_claims)).is_err(),
            "accepted {field}"
        );
    }
    let mut wrong = config();
    wrong.token_audience = "nvbes-billing-service".into();
    assert!(TokenVerifier::new(&wrong).is_err());
}

#[test]
fn sensitive_actions_require_current_proof_not_a_historical_mfa_method() {
    let verifier = TokenVerifier::new(&config()).unwrap();
    let mut claims = claims();
    let now = claims["iat"].as_u64().unwrap();
    claims["iat"] = json!(now - 120);
    claims["nbf"] = json!(now - 120);
    claims["auth_time"] = json!(now - 200);
    claims["step_up_time"] = json!(now - 150);
    claims["step_up_expires_at"] = json!(now - 1);
    assert!(
        verifier
            .validated(&sign(&claims))
            .unwrap()
            .0
            .require_step_up()
            .is_err()
    );
    claims.as_object_mut().unwrap().remove("step_up_time");
    claims.as_object_mut().unwrap().remove("step_up_expires_at");
    assert!(
        verifier
            .validated(&sign(&claims))
            .unwrap()
            .0
            .require_step_up()
            .is_err()
    );
    claims["amr"] = json!(["webauthn"]);
    let mut principal = verifier.validated(&sign(&claims)).unwrap().0;
    assert!(principal.require_step_up().is_ok());
    principal.strong_until = Some(now - 1);
    assert!(principal.require_step_up().is_err());
    claims["auth_time"] = json!(now - 301);
    assert!(
        verifier
            .validated(&sign(&claims))
            .unwrap()
            .0
            .require_step_up()
            .is_err()
    );
}

#[tokio::test]
async fn account_http_guard_requires_live_identity_and_refuses_duplicate_headers() {
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
    let mut config = config();
    config.token_issuer = issuer.clone();
    let mut claims = claims();
    claims["iss"] = json!(issuer);
    let token = sign(&claims);
    let mut active = claims;
    active["active"] = json!(true);
    active["token_type"] = json!("Bearer");
    let response = Arc::new(Mutex::new((StatusCode::OK, active)));
    let calls = Arc::new(AtomicUsize::new(0));
    let identity = Router::new().route(
        "/oauth/introspect",
        post({
            let response = response.clone();
            let calls = calls.clone();
            move || {
                let (status, body) = response.lock().unwrap().clone();
                calls.fetch_add(1, Ordering::SeqCst);
                async move { (status, Json(body)) }
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, identity).await.unwrap();
    });
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy(&config.database_url)
        .unwrap();
    let state = crate::app::AccountState::new(config, db).unwrap();
    let reached = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route(
            "/sensitive",
            get({
                let reached = reached.clone();
                move |principal: Principal| {
                    let reached = reached.clone();
                    async move {
                        principal.require("account:close")?;
                        principal.require_step_up()?;
                        reached.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, AccountError>(StatusCode::NO_CONTENT)
                    }
                }
            }),
        )
        .with_state(state);
    let request = || {
        Request::get("/sensitive")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    };
    assert_eq!(
        app.clone().oneshot(request()).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
    *response.lock().unwrap() = (StatusCode::OK, json!({"active":false}));
    assert_eq!(
        app.clone().oneshot(request()).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    *response.lock().unwrap() = (StatusCode::SERVICE_UNAVAILABLE, json!({}));
    assert_eq!(
        app.clone().oneshot(request()).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let mut duplicate = request();
    duplicate
        .headers_mut()
        .append("authorization", format!("Bearer {token}").parse().unwrap());
    assert_eq!(
        app.oneshot(duplicate).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(reached.load(Ordering::SeqCst), 1);
    server.abort();
}
