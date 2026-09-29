//! Shared HTTP + JWT fixtures for Account service integration tests.

use axum::{Form, Json, Router, routing::post};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use openssl::{pkey::PKey, rsa::Rsa};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::{app::AccountState, auth::TokenVerifier, config::AccountConfig};

/// Serializes process-wide env mutation across Account unit tests.
pub(crate) fn test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TestKeys {
    private_pem: String,
    public_pem: String,
}

fn test_keys() -> &'static TestKeys {
    static KEYS: OnceLock<TestKeys> = OnceLock::new();
    KEYS.get_or_init(|| {
        let private = PKey::from_rsa(Rsa::generate(2048).expect("rsa")).expect("pkey");
        TestKeys {
            private_pem: String::from_utf8(private.private_key_to_pem_pkcs8().expect("priv pem"))
                .expect("utf8"),
            public_pem: String::from_utf8(private.public_key_to_pem().expect("pub pem"))
                .expect("utf8"),
        }
    })
}

pub(crate) fn account_config() -> AccountConfig {
    AccountConfig {
        browser_origins: Default::default(),
        public_origin: None,
        billing_authorization_secret: None,
        token_verification_keys: "[]".into(),
        identity_resource_client_id: "account-test".into(),
        identity_resource_secret: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
        environment: "test".into(),
        database_url: "postgres://unused".into(),
        database_max_connections: 1,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        token_issuer: "http://127.0.0.1".into(),
        token_audience: "nvbes-account-service".into(),
        token_key_id: "identity-key-1".into(),
        token_public_key_pem: test_keys().public_pem.clone(),
        metrics_token: "development-account-metrics-token-value".into(),
        sentry_dsn: None,
        sentry_traces_sample_rate: 0.1,
        otlp_endpoint: None,
        otlp_authorization_header: None,
    }
}

pub(crate) struct HttpHarness {
    pub state: AccountState,
    issuer: String,
    replies: Arc<Mutex<HashMap<String, Value>>>,
    _calls: Arc<AtomicUsize>,
    _server: JoinHandle<()>,
}

impl HttpHarness {
    pub(crate) async fn new(pool: PgPool) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let replies = Arc::new(Mutex::new(HashMap::<String, Value>::new()));
        let calls = Arc::new(AtomicUsize::new(0));
        let identity = Router::new().route(
            "/oauth/introspect",
            post({
                let replies = replies.clone();
                let calls = calls.clone();
                move |Form(body): Form<HashMap<String, String>>| {
                    let replies = replies.clone();
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        let token = body.get("token").cloned().unwrap_or_default();
                        let body = replies
                            .lock()
                            .expect("introspection replies")
                            .get(&token)
                            .cloned()
                            .unwrap_or_else(|| json!({"active": false}));
                        Json(body)
                    }
                }
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, identity).await.unwrap();
        });
        let mut config = account_config();
        config.token_issuer = issuer.clone();
        let state = AccountState::new(config, pool).expect("account state");
        Self {
            state,
            issuer,
            replies,
            _calls: calls,
            _server: server,
        }
    }

    pub(crate) fn access_token(&self, principal_id: Uuid, scopes: &str, step_up: bool) -> String {
        let now = jsonwebtoken::get_current_timestamp();
        let mut amr = vec!["pwd"];
        if step_up {
            amr.push("totp");
        }
        let mut claims = json!({
            "sub": principal_id,
            "sid": Uuid::new_v4(),
            "grant_id": Uuid::new_v4(),
            "jti": Uuid::new_v4(),
            "iss": self.issuer,
            "aud": "nvbes-account-service",
            "client_id": "account-web",
            "token_type": "access",
            "scope": scopes,
            "amr": amr,
            "auth_time": now.saturating_sub(60),
            "iat": now,
            "nbf": now,
            "exp": now + 600,
        });
        if step_up {
            claims["step_up_time"] = json!(now.saturating_sub(30));
            claims["step_up_expires_at"] = json!(now + 300);
        }
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("identity-key-1".into());
        header.typ = Some("at+jwt".into());
        let key = EncodingKey::from_rsa_pem(test_keys().private_pem.as_bytes()).expect("encoding");
        let token = encode(&header, &claims, &key).unwrap();
        let mut active = claims;
        active["active"] = json!(true);
        active["token_type"] = json!("Bearer");
        self.replies
            .lock()
            .expect("introspection replies")
            .insert(token.clone(), active);
        token
    }
}

/// Back-compat helper for tests that only need a pool-backed state without HTTP auth.
pub(crate) fn state_with_pool(pool: PgPool) -> AccountState {
    AccountState::new(account_config(), pool).expect("account state")
}

#[allow(
    dead_code,
    reason = "shared JWT verifier helper for cross-module account tests"
)]
pub(crate) fn verifier() -> TokenVerifier {
    TokenVerifier::new(&account_config()).unwrap()
}
