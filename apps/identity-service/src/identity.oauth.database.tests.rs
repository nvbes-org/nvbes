use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use super::{
    clients::ClientRegistry,
    codes::{self, CodeExchange},
    request::AuthorizationInput,
    store::{self, RequestKind},
};

const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

async fn database() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must name an isolated Identity test database");
    let parsed = reqwest::Url::parse(&url).unwrap();
    assert!(matches!(
        parsed.host_str(),
        Some("localhost" | "127.0.0.1" | "[::1]")
    ));
    assert!(parsed.path().contains("identity_test"));
    let db = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&db).await.unwrap();
    db
}

fn clients() -> ClientRegistry {
    ClientRegistry::from_json(
        r#"[{
        "client_id":"account-web","display_name":"Account",
        "redirect_uris":["https://account.example/callback"],"post_logout_redirect_uris":[],
        "resources":{"https://api.example/account":{"audience":"nvbes-account-service","scopes":["account:read"]}},
        "allow_refresh":true,"require_dpop":false
    }]"#,
        false,
    )
    .unwrap()
}

async fn request(db: &PgPool, clients: &ClientRegistry, kind: RequestKind) -> String {
    let input = AuthorizationInput {
        client_id: "account-web".into(),
        redirect_uri: "https://account.example/callback".into(),
        response_type: "code".into(),
        scope: "openid account:read".into(),
        resource: "https://api.example/account".into(),
        state: "random-state-for-test".into(),
        nonce: "random-nonce-for-test".into(),
        code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into(),
        code_challenge_method: "S256".into(),
        dpop_jkt: None,
        max_age: None,
        prompt: None,
    }
    .validate(clients)
    .unwrap();
    store::create_request(db, &input, kind).await.unwrap()
}

async fn session(db: &PgPool) -> (Uuid, String) {
    let principal = Uuid::new_v4();
    let session = Uuid::new_v4();
    let token = store::random_secret();
    sqlx::query("INSERT INTO identity_principals(id,kind,status) VALUES($1,'human','active')")
        .bind(principal)
        .execute(db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO identity_sessions(id,principal_id,token_hash,expires_at) VALUES($1,$2,$3,clock_timestamp()+interval '1 hour')")
        .bind(session).bind(principal).bind(store::hash(&token)).execute(db).await.unwrap();
    (session, token)
}

fn exchange(code: &str) -> CodeExchange<'_> {
    CodeExchange {
        code,
        client_id: "account-web",
        redirect_uri: "https://account.example/callback",
        verifier: VERIFIER,
        verified_dpop_jkt: None,
    }
}

#[tokio::test]
async fn par_is_bound_to_client_and_consumed_once_under_concurrency() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Par).await;
    assert!(
        store::consume_par(&db, &clients, &handle, "other-client")
            .await
            .is_err()
    );
    let (a, b) = tokio::join!(
        store::consume_par(&db, &clients, &handle, "account-web"),
        store::consume_par(&db, &clients, &handle, "account-web")
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let next = a.or(b).unwrap();
    assert!(store::load_request(&db, &clients, &next).await.is_ok());
    let raw_stored: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_oauth_requests WHERE handle_hash=convert_to($1,'UTF8'))")
        .bind(&next).fetch_one(&db).await.unwrap();
    assert!(!raw_stored);
}

#[tokio::test]
async fn exchange_binds_pkce_client_redirect_and_revokes_grant_on_valid_replay() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (_, token) = session(&db).await;
    let code = codes::authorize(&db, &clients, &handle, &token)
        .await
        .unwrap()
        .code;
    assert!(
        codes::authorize(&db, &clients, &handle, &token)
            .await
            .is_err()
    );
    let mut bad = exchange(&code);
    bad.verifier = "invalid";
    assert!(codes::exchange(&db, &clients, bad).await.is_err());
    let mut bad = exchange(&code);
    bad.redirect_uri = "https://evil.example/";
    assert!(codes::exchange(&db, &clients, bad).await.is_err());
    let mut bad = exchange(&code);
    bad.client_id = "other-client";
    assert!(codes::exchange(&db, &clients, bad).await.is_err());
    let grant = codes::exchange(&db, &clients, exchange(&code))
        .await
        .unwrap();
    sqlx::query("UPDATE identity_oauth_codes SET created_at=clock_timestamp()-interval '10 minutes',expires_at=clock_timestamp()-interval '5 minutes' WHERE code_hash=$1")
        .bind(store::hash(&code)).execute(&db).await.unwrap();
    assert!(
        codes::exchange(&db, &clients, exchange(&code))
            .await
            .is_err()
    );
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity_oauth_grants WHERE id=$1")
            .bind(grant.id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(revoked);
}

#[tokio::test]
async fn expired_requests_and_revoked_sessions_cannot_authorize() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    sqlx::query("UPDATE identity_oauth_requests SET created_at=clock_timestamp()-interval '10 minutes',expires_at=clock_timestamp()-interval '5 minutes' WHERE handle_hash=$1")
        .bind(store::hash(&handle)).execute(&db).await.unwrap();
    assert!(store::load_request(&db, &clients, &handle).await.is_err());
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (id, token) = session(&db).await;
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(id)
        .execute(&db)
        .await
        .unwrap();
    assert!(
        codes::authorize(&db, &clients, &handle, &token)
            .await
            .is_err()
    );
    assert!(store::load_request(&db, &clients, &handle).await.is_ok());
}

#[tokio::test]
async fn concurrent_code_exchanges_issue_only_one_grant() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (_, token) = session(&db).await;
    let code = codes::authorize(&db, &clients, &handle, &token)
        .await
        .unwrap()
        .code;
    let (a, b) = tokio::join!(
        codes::exchange(&db, &clients, exchange(&code)),
        codes::exchange(&db, &clients, exchange(&code))
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let grant = a.or(b).unwrap();
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity_oauth_grants WHERE id=$1")
            .bind(grant.id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(
        revoked,
        "a valid duplicate exchange revokes the single resulting grant"
    );
}
