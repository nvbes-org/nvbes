use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::oauth::{
    clients::ClientRegistry,
    codes::CodeExchange,
    request::AuthorizationInput,
    store::{self, RequestKind},
};

const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

pub async fn database() -> PgPool {
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

pub fn clients() -> ClientRegistry {
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

pub async fn request(db: &PgPool, clients: &ClientRegistry, kind: RequestKind) -> String {
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

pub async fn session(db: &PgPool) -> (Uuid, String) {
    let principal = Uuid::new_v4();
    let session = Uuid::new_v4();
    let token = store::random_secret();
    sqlx::query("INSERT INTO identity_principals(id,kind,status) VALUES($1,'human','active')")
        .bind(principal)
        .execute(db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO identity_sessions(id,principal_id,token_hash,expires_at,authenticated_at,primary_amr) VALUES($1,$2,$3,clock_timestamp()+interval '1 hour',clock_timestamp(),'pwd')")
        .bind(session).bind(principal).bind(store::hash(&token)).execute(db).await.unwrap();
    (session, token)
}

pub fn exchange(code: &str) -> CodeExchange<'_> {
    CodeExchange {
        code,
        client_id: "account-web",
        redirect_uri: "https://account.example/callback",
        verifier: VERIFIER,
        verified_dpop_jkt: None,
    }
}
