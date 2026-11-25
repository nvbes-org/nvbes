use super::*;
use jsonwebtoken::{EncodingKey, Header, encode};
use nvbes_identity_sdk::introspection::IntrospectionError;
use rsa::{
    RsaPrivateKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    rand_core::OsRng,
};
use serde_json::{Value, json};
use std::sync::OnceLock;
#[path = "account.auth.key_rotation.tests.rs"]
mod key_rotation;
#[cfg(feature = "database-tests")]
#[path = "account.privacy.deadline.tests.rs"]
mod privacy_deadline;

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
fn trust() -> TokenTrust {
    TokenTrust {
        issuer: "https://identity.example/".into(),
        key_id: "identity-test".into(),
        public_key_pem: keys().1.clone(),
        verification_keys: "[]".into(),
    }
}

/// Activity double whose answer the test switches between calls.
#[derive(Clone)]
struct ScriptedActivity {
    answer: std::sync::Arc<std::sync::Mutex<Result<Value, IntrospectionError>>>,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl ScriptedActivity {
    fn new(answer: Value) -> Self {
        Self {
            answer: std::sync::Arc::new(std::sync::Mutex::new(Ok(answer))),
            calls: Default::default(),
        }
    }

    fn set(&self, answer: Result<Value, IntrospectionError>) {
        *self.answer.lock().unwrap() = answer;
    }
}

#[async_trait::async_trait]
impl ActivityVerifier for ScriptedActivity {
    async fn is_active(
        &self,
        _token: &str,
        expected: &ExpectedToken,
    ) -> Result<bool, IntrospectionError> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let answer = match &*self.answer.lock().unwrap() {
            Ok(value) => value.clone(),
            Err(_) => return Err(IntrospectionError::Unavailable),
        };
        if answer["active"] != json!(true) {
            return Ok(false);
        }
        let actual: ExpectedToken =
            serde_json::from_value(answer).map_err(|_| IntrospectionError::Unavailable)?;
        nvbes_identity_sdk::introspection::confirm_activity(&actual, expected)
    }
}

fn verifier() -> TokenVerifier {
    TokenVerifier::new(
        &trust(),
        None,
        std::sync::Arc::new(ScriptedActivity::new(json!({"active": false}))),
    )
    .unwrap()
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
    let verifier = verifier();
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
    let mut wrong = trust();
    wrong.public_key_pem = "not a key".into();
    assert!(
        TokenVerifier::new(
            &wrong,
            None,
            std::sync::Arc::new(ScriptedActivity::new(json!({"active": false})))
        )
        .is_err()
    );
}

#[test]
fn sensitive_actions_require_current_proof_not_a_historical_mfa_method() {
    let verifier = verifier();
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
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tower::ServiceExt;
    let claims = claims();
    let token = sign(&claims);
    let mut active = claims;
    active["active"] = json!(true);
    active["token_type"] = json!("Bearer");
    let activity = ScriptedActivity::new(active);
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1/unused")
        .unwrap();
    let state = crate::app::AccountState::new(
        crate::test_support::account_config(),
        trust(),
        db,
        Arc::new(activity.clone()),
    )
    .unwrap();
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
    activity.set(Ok(json!({"active": false})));
    assert_eq!(
        app.clone().oneshot(request()).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    activity.set(Err(IntrospectionError::Unavailable));
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
    assert_eq!(activity.calls.load(Ordering::SeqCst), 3);
    assert_eq!(reached.load(Ordering::SeqCst), 1);
}
