//! Interactive browser harness; never compiled into the production runtime.
use crate::{
    browser::BrowserSecurity,
    oauth::{clients::ClientRegistry, http},
    tokens::TokenService,
    tokens_config::TokenConfig,
};
use axum::{
    Json, Router,
    extract::Query,
    response::Html,
    routing::{get, post},
};
use std::{collections::HashMap, sync::Arc, time::Duration};

#[tokio::test]
#[ignore = "interactive Playwright fixture: launch with identity-service:test:browser-fixture"]
async fn browser_e2e_fixture() {
    let db = crate::test_fixtures::isolated_database().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://localhost:{}", listener.local_addr().unwrap().port());
    let email = "browser-fixture@example.invalid";
    let password = "Browser-fixture-only-password-97!";
    let principal = crate::auth::create_synthetic_identity(&db, email, password)
        .await
        .unwrap();
    let old = crate::tokens::tests::config();
    let config = TokenConfig::from_values(
        "test",
        origin.clone(),
        old.key_id,
        old.private_key_pem,
        old.public_key_pem,
        "nvbes-identity-userinfo".into(),
    )
    .unwrap();
    let tokens = Arc::new(TokenService::new(config).unwrap());
    let clients = Arc::new(ClientRegistry::from_json(&serde_json::json!([{
        "client_id":"browser-fixture","display_name":"Browser fixture",
        "redirect_uris":[format!("{origin}/callback")],"post_logout_redirect_uris":[],
        "resources":{format!("{origin}/oauth/userinfo"):{"audience":"nvbes-identity-userinfo","scopes":["openid","email"]}},
        "allow_refresh":false,"require_dpop":false
    }]).to_string(),true).unwrap());
    let browser = BrowserSecurity::new(&origin, true).unwrap();
    let webauthn = Arc::new(crate::webauthn::build_server("localhost", &origin).unwrap());
    let limiter = crate::rate_limits::RateLimiter::new(rand::random()).unwrap();
    let mfa = Arc::new(crate::mfa_crypto::MfaCrypto::with_rotation(1, [21; 32], None).unwrap());
    let (done, receiver) = tokio::sync::oneshot::channel();
    let done = Arc::new(tokio::sync::Mutex::new(Some(done)));
    let metadata = serde_json::json!({"email":email,"password":password,"client_id":"browser-fixture","principal":principal});
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html("<!doctype html><title>Identity browser test fixture</title>") }),
        )
        .route(
            "/__fixture/bootstrap",
            get(move || {
                let metadata = metadata.clone();
                async move { Json(metadata) }
            }),
        )
        .route(
            "/__fixture/done",
            post(move || {
                let done = done.clone();
                async move {
                    if let Some(done) = done.lock().await.take() {
                        let _ = done.send(());
                    }
                    Json(serde_json::json!({"done":true}))
                }
            }),
        )
        .route(
            "/callback",
            get(|Query(query): Query<HashMap<String, String>>| async move { Json(query) }),
        )
        .merge(http::router(&origin, tokens.clone()))
        .merge(http::token_router(
            db.clone(),
            clients.clone(),
            tokens,
            limiter.clone(),
        ))
        .merge(http::authorization_router(
            db.clone(),
            clients.clone(),
            browser.clone(),
            mfa,
            limiter.clone(),
        ))
        .merge(http::passkey_login_router(
            db.clone(),
            clients,
            browser.clone(),
            webauthn.clone(),
            limiter.clone(),
        ))
        .merge(http::webauthn_router(
            db.clone(),
            browser,
            webauthn,
            limiter,
        ));
    println!("BROWSER_FIXTURE_ORIGIN={origin}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::time::timeout(Duration::from_secs(180), receiver)
            .await
            .expect("browser fixture timed out")
            .expect("browser completion signal missing");
    })
    .await
    .unwrap();
    let count: i64=sqlx::query_scalar("SELECT count(*) FROM identity_sessions WHERE principal_id=$1 AND primary_amr='webauthn' AND revoked_at IS NULL")
        .bind(principal).fetch_one(&db).await.unwrap();
    assert_eq!(count, 1);
    let grants: i64=sqlx::query_scalar("SELECT count(*) FROM identity_oauth_grants WHERE principal_id=$1 AND token_issued_at IS NOT NULL")
        .bind(principal).fetch_one(&db).await.unwrap();
    assert_eq!(grants, 1);
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(schema.starts_with("identity_test_"));
    assert!(
        schema
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    );
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&db)
        .await
        .unwrap();
    db.close().await;
}
