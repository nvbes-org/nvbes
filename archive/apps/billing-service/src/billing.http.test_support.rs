//! Shared HTTP router and JWT fixtures for billing integration tests.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::{Form, Json, Router, routing::post};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use openssl::{pkey::PKey, rsa::Rsa};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::{
    app::{BillingState, create_router},
    auth::TokenVerifier,
    authorization::AccountAuthority,
    config::BillingConfig,
    database::test_support::test_config,
    metrics::install,
};

const ACCOUNT_AUTHORITY_SECRET: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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

async fn spawn_account_authority() -> (AccountAuthority, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let app = Router::new().route(
        "/internal/v1/billing/authorize",
        post(|Json(mut body): Json<Value>| async move {
            body["allowed"] = json!(true);
            Json(body)
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let authority =
        AccountAuthority::new(&origin, ACCOUNT_AUTHORITY_SECRET).expect("account authority");
    (authority, server)
}

pub struct JwtHarness {
    pub router: Router,
    issuer: String,
    replies: Arc<Mutex<HashMap<String, Value>>>,
    _calls: Arc<AtomicUsize>,
    _identity: JoinHandle<()>,
    _account: JoinHandle<()>,
}

impl JwtHarness {
    pub async fn new(pool: PgPool) -> Self {
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
        let identity_server = tokio::spawn(async move {
            axum::serve(listener, identity).await.unwrap();
        });
        let (account_authority, account_server) = spawn_account_authority().await;
        let mut config = test_config();
        config.account_authority = Some(account_authority);
        config.identity_public_key_pem = Some(test_keys().public_pem.clone());
        config.identity_token_issuer = Some(issuer.clone());
        config.identity_token_key_id = Some("identity-key-1".into());
        config.identity_resource_client_id = Some("billing-test".into());
        config.identity_resource_secret =
            Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into());
        let tokens = TokenVerifier::new(&config).expect("verifier");
        let state = BillingState {
            db: pool,
            config,
            metrics: install(),
            tokens,
            email_client: None,
        };
        Self {
            router: create_router(state),
            issuer,
            replies,
            _calls: calls,
            _identity: identity_server,
            _account: account_server,
        }
    }

    pub fn bearer_jwt(&self, principal_id: Uuid, scopes: &str) -> String {
        format!("Bearer {}", self.access_token(principal_id, scopes))
    }

    fn access_token(&self, principal_id: Uuid, scopes: &str) -> String {
        let now = jsonwebtoken::get_current_timestamp();
        let mut claims = json!({
            "sub": principal_id,
            "iss": self.issuer,
            "aud": "nvbes-billing-service",
            "token_type": "access",
            "scope": scopes,
            "amr": ["pwd"],
            "client_id": "account-web",
            "exp": now + 600,
            "iat": now,
            "nbf": now,
            "auth_time": now.saturating_sub(60),
            "jti": Uuid::new_v4(),
            "sid": Uuid::new_v4(),
            "grant_id": Uuid::new_v4(),
        });
        let mut header = Header::new(Algorithm::RS256);
        header.typ = Some("at+jwt".into());
        header.kid = Some("identity-key-1".into());
        let key = EncodingKey::from_rsa_pem(test_keys().private_pem.as_bytes()).expect("encoding");
        let token = encode(&header, &claims, &key).expect("jwt");
        claims["active"] = json!(true);
        claims["token_type"] = json!("Bearer");
        self.replies
            .lock()
            .expect("introspection replies")
            .insert(token.clone(), claims);
        token
    }
}

/// HTTP router with UUID bearer bypass and a permissive Account authority fixture.
pub struct TestApp {
    pub router: Router,
    _account: JoinHandle<()>,
}

impl TestApp {
    pub async fn new(pool: PgPool) -> Self {
        Self::with_config(pool, test_config()).await
    }

    pub async fn with_config(pool: PgPool, mut config: BillingConfig) -> Self {
        let (account_authority, account_server) = spawn_account_authority().await;
        if config.account_authority.is_none() {
            config.account_authority = Some(account_authority);
        }
        let tokens = TokenVerifier::new(&config).expect("verifier");
        let state = BillingState {
            db: pool,
            config,
            metrics: install(),
            tokens,
            email_client: None,
        };
        Self {
            router: create_router(state),
            _account: account_server,
        }
    }
}

pub fn bearer(principal_id: Uuid) -> String {
    format!("Bearer {}", principal_id)
}

pub fn test_router(pool: PgPool) -> Router {
    // Prefer `TestApp::new` so the Account authority fixture stays alive for the request.
    // Kept for non-authorization code paths (health, unauthenticated).
    test_router_with_config(pool, test_config())
}

pub fn test_router_with_config(pool: PgPool, config: BillingConfig) -> Router {
    let tokens = TokenVerifier::new(&config).expect("verifier");
    let state = BillingState {
        db: pool,
        config,
        metrics: install(),
        tokens,
        email_client: None,
    };
    create_router(state)
}
