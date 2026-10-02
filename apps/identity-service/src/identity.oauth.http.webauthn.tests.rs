use super::*;
use crate::test_fixtures::{isolated_database, session};
use axum::{
    body::{Body, to_bytes},
    http::{Request as HttpRequest, StatusCode},
};
use tower::ServiceExt;
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};

const PATHS: [&str; 4] = [
    "registration/options",
    "registration/finish",
    "step-up/options",
    "step-up/finish",
];
struct Fixture {
    db: PgPool,
    principal: uuid::Uuid,
    app: Router,
    cookie: String,
    csrf: String,
    limiter: RateLimiter,
}
impl Fixture {
    async fn new() -> Self {
        let db = isolated_database().await;
        let (session_id, token) = session(&db).await;
        let principal =
            sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
                .bind(session_id)
                .fetch_one(&db)
                .await
                .unwrap();
        let browser = BrowserSecurity::new("https://identity.example", false).unwrap();
        let browser_token = browser.browser_cookie().token;
        let csrf = browser.session_csrf_token(&token, &browser_token).unwrap();
        let cookie = format!("__Host-nvbes-browser={browser_token}; __Host-nvbes-session={token}");
        let limiter = RateLimiter::new(rand::random()).unwrap();
        let server = Arc::new(
            crate::webauthn::build_server("identity.example", "https://identity.example").unwrap(),
        );
        let app = router(db.clone(), browser, server, limiter.clone());
        Self {
            db,
            principal,
            app,
            cookie,
            csrf,
            limiter,
        }
    }
    async fn send(
        &self,
        path: &str,
        body: String,
        origin: &str,
        csrf: &str,
    ) -> (StatusCode, serde_json::Value) {
        let request = HttpRequest::builder()
            .method("POST")
            .uri(format!("/oauth/session/webauthn/{path}"))
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:7000".parse::<std::net::SocketAddr>().unwrap(),
            ))
            .header("origin", origin)
            .header("content-type", "application/json")
            .header("cookie", &self.cookie)
            .header("x-csrf-token", csrf)
            .body(Body::from(body))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        let status = response.status();
        let json =
            serde_json::from_slice(&to_bytes(response.into_body(), 131072).await.unwrap()).unwrap();
        (status, json)
    }
    async fn post(&self, path: &str, body: serde_json::Value) -> serde_json::Value {
        let (status, json) = self
            .send(
                path,
                body.to_string(),
                "https://identity.example",
                &self.csrf,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{json}");
        json
    }
}

#[tokio::test]
async fn browser_http_can_enroll_and_step_up_with_a_signed_assertion() {
    let f = Fixture::new().await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let started = f.post(PATHS[0], serde_json::json!({})).await;
    let credential = authenticator
        .do_registration(
            "https://identity.example".parse().unwrap(),
            serde_json::from_value(started["options"].clone()).unwrap(),
        )
        .unwrap();
    let enrolled = f.post(PATHS[1],serde_json::json!({"ceremony_id":started["ceremony_id"],"credential":credential,"label":"Browser test"})).await;
    assert!(enrolled["credential_id"].is_string());
    let started = f.post(PATHS[2], serde_json::json!({})).await;
    let credential = authenticator
        .do_authentication(
            "https://identity.example".parse().unwrap(),
            serde_json::from_value(started["options"].clone()).unwrap(),
        )
        .unwrap();
    let body = serde_json::json!({"ceremony_id":started["ceremony_id"],"credential":credential});
    let finished = f.post(PATHS[3], body.clone()).await;
    assert_eq!(finished["step_up"], true);
    assert_eq!(
        f.send(
            PATHS[3],
            body.to_string(),
            "https://identity.example",
            &f.csrf
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn every_route_rejects_bad_origin_and_csrf_before_parsing() {
    let f = Fixture::new().await;
    for path in PATHS {
        assert_eq!(
            f.send(path, "not json".into(), "https://evil.example", &f.csrf)
                .await
                .0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            f.send(
                path,
                "not json".into(),
                "https://identity.example",
                &crate::oauth::store::random_secret()
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_webauthn_challenges")
        .fetch_one(&f.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn source_quota_blocks_all_routes_before_browser_parsing() {
    let f = Fixture::new().await;
    for _ in 0..30 {
        f.limiter
            .check(&f.db, Category::LoginSource, "127.0.0.1")
            .await
            .unwrap();
    }
    for path in PATHS {
        assert_eq!(
            f.send(path, "bad".into(), "https://evil.example", "bad")
                .await
                .0,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
}

#[tokio::test]
async fn malformed_bodies_are_bounded_and_account_quota_persists_failures() {
    let f = Fixture::new().await;
    for body in [
        "not-json".into(),
        "x".repeat(65537),
        "{\"unrecognized\":true}".into(),
        "[]".into(),
        "null".into(),
    ] {
        let result = f
            .send(PATHS[0], body, "https://identity.example", &f.csrf)
            .await;
        assert_eq!(result.0, StatusCode::BAD_REQUEST);
        assert_eq!(result.1, serde_json::json!({"error":"invalid_request"}));
    }
    for _ in 5..20 {
        assert_eq!(
            f.send(PATHS[0], "null".into(), "https://identity.example", &f.csrf)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        f.send(PATHS[0], "{}".into(), "https://identity.example", &f.csrf)
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn webauthn_has_a_separate_budget_without_resetting_totp_attempts() {
    let f = Fixture::new().await;
    let subject = f.principal.to_string();
    for _ in 0..5 {
        f.limiter
            .check(&f.db, Category::MfaAccount, &subject)
            .await
            .unwrap();
    }
    // Even an exhausted TOTP budget permits a cryptographic registration ceremony.
    f.post(PATHS[0], serde_json::json!({})).await;
    assert!(matches!(
        f.limiter.check(&f.db, Category::MfaAccount, &subject).await,
        Err(crate::rate_limits::LimitError::Exceeded)
    ));
    // The account budget is independent of source limits and shared by sessions.
    for _ in 1..20 {
        f.limiter
            .check(&f.db, Category::WebauthnAccount, &subject)
            .await
            .unwrap();
    }
    for path in PATHS {
        assert_eq!(
            f.send(path, "{}".into(), "https://identity.example", &f.csrf)
                .await
                .0,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
}

#[tokio::test]
async fn unavailable_quota_store_is_not_bypassed() {
    let f = Fixture::new().await;
    sqlx::query("ALTER TABLE identity_rate_buckets RENAME TO unavailable_buckets")
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.send(PATHS[0], "{}".into(), "https://identity.example", &f.csrf)
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
}
