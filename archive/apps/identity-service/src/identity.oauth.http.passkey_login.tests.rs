use super::*;
use crate::{
    oauth::{interactions, store},
    test_fixtures::{self, isolated_database, session},
    webauthn::registration,
};
use axum::{
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use tower::ServiceExt;
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};

const OPTIONS: &str = "/oauth/authorize/passkey/options";
const FINISH: &str = "/oauth/authorize/passkey/finish";
struct Fixture {
    db: PgPool,
    app: Router,
    browser: String,
    csrf: String,
    handle: String,
    principal: uuid::Uuid,
    credential: Vec<u8>,
    authenticator: WebauthnAuthenticator<SoftPasskey>,
    limiter: RateLimiter,
}
impl Fixture {
    async fn new() -> Self {
        let db = isolated_database().await;
        let (id, token) = session(&db).await;
        let principal =
            sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
                .bind(id)
                .fetch_one(&db)
                .await
                .unwrap();
        let server = Arc::new(
            crate::webauthn::build_server("identity.example", "https://identity.example").unwrap(),
        );
        let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let started = registration::start(&db, &server, &token).await.unwrap();
        let response = authenticator
            .do_registration("https://identity.example".parse().unwrap(), started.options)
            .unwrap();
        let id = registration::finish(&db, &server, &token, started.ceremony_id, &response, "Test")
            .await
            .unwrap();
        let credential = sqlx::query_scalar(
            "SELECT credential_id FROM identity_webauthn_credentials WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&db)
        .await
        .unwrap();
        let clients = Arc::new(test_fixtures::clients());
        let handle = test_fixtures::request(&db, &clients, store::RequestKind::Authorization).await;
        let browser = store::random_secret();
        let csrf = interactions::begin(&db, &clients, &handle, &browser, None)
            .await
            .unwrap()
            .csrf_token;
        let security = BrowserSecurity::new("https://identity.example", false).unwrap();
        let limiter = RateLimiter::new(rand::random()).unwrap();
        let app = router(
            db.clone(),
            clients.clone(),
            security.clone(),
            server,
            limiter.clone(),
        )
        .merge(super::super::authorization_router(
            db.clone(),
            clients,
            security,
            Arc::new(crate::mfa_crypto::MfaCrypto::with_rotation(1, [17; 32], None).unwrap()),
            limiter.clone(),
        ));
        Self {
            db,
            app,
            browser,
            csrf,
            handle,
            principal,
            credential,
            authenticator,
            limiter,
        }
    }
    async fn send(
        &self,
        path: &str,
        body: String,
        origin: &str,
        csrf: &str,
        session: Option<&str>,
    ) -> (StatusCode, HeaderMap, serde_json::Value) {
        let mut cookie = format!("__Host-nvbes-browser={}", self.browser);
        if let Some(session) = session {
            cookie.push_str(&format!("; __Host-nvbes-session={session}"));
        }
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .extension(axum::extract::ConnectInfo(
                        "127.0.0.1:8000".parse::<std::net::SocketAddr>().unwrap(),
                    ))
                    .header("origin", origin)
                    .header("content-type", "application/json")
                    .header("cookie", cookie)
                    .header("x-csrf-token", csrf)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), 131072).await.unwrap();
        let json = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, headers, json)
    }
    async fn assertion(&mut self) -> serde_json::Value {
        let (status, headers, started) = self
            .send(
                OPTIONS,
                serde_json::json!({"interaction":self.handle}).to_string(),
                "https://identity.example",
                &self.csrf,
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(!headers.contains_key("set-cookie"));
        let mut options = started["options"].clone();
        // SoftPasskey lacks discoverable UI; emulate selection only on the client.
        options["publicKey"]["allowCredentials"] = serde_json::json!([{"type":"public-key","id":URL_SAFE_NO_PAD.encode(&self.credential)}]);
        let response = self
            .authenticator
            .do_authentication(
                "https://identity.example".parse().unwrap(),
                serde_json::from_value(options).unwrap(),
            )
            .unwrap();
        let mut response = serde_json::to_value(response).unwrap();
        response["response"]["userHandle"] =
            URL_SAFE_NO_PAD.encode(self.principal.as_bytes()).into();
        serde_json::json!({"interaction":self.handle,"ceremony_id":started["ceremony_id"],"credential":response})
    }
}

#[tokio::test]
async fn http_passkey_login_sets_host_cookie_and_can_approve_oauth() {
    let mut f = Fixture::new().await;
    let body = f.assertion().await;
    let (status, headers, json) = f
        .send(
            FINISH,
            body.to_string(),
            "https://identity.example",
            &f.csrf,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let cookie = headers["set-cookie"].to_str().unwrap();
    for part in [
        "__Host-nvbes-session=",
        "HttpOnly",
        "Secure",
        "SameSite=Lax",
        "Path=/",
    ] {
        assert!(cookie.contains(part));
    }
    let token = cookie.split(';').next().unwrap().split_once('=').unwrap().1;
    assert!(json["session_csrf_token"].is_string());
    assert_ne!(json["csrf_token"], f.csrf);
    assert!(json.get("token").is_none());
    let approved = f
        .send(
            "/oauth/authorize/approve",
            serde_json::json!({"interaction":f.handle}).to_string(),
            "https://identity.example",
            json["csrf_token"].as_str().unwrap(),
            Some(token),
        )
        .await;
    assert_eq!(approved.0, StatusCode::SEE_OTHER);
    assert!(approved.1["location"].to_str().unwrap().contains("code="));
    let replay = f
        .send(
            FINISH,
            body.to_string(),
            "https://identity.example",
            &f.csrf,
            None,
        )
        .await;
    assert_eq!(replay.0, StatusCode::BAD_REQUEST);
    assert!(!replay.1.contains_key("set-cookie"));
}

#[tokio::test]
async fn bad_browser_body_and_source_quota_never_issue_cookies() {
    let f = Fixture::new().await;
    for path in [OPTIONS, FINISH] {
        let bad = f
            .send(
                path,
                "invalid".into(),
                "https://evil.example",
                &f.csrf,
                None,
            )
            .await;
        assert_eq!(bad.0, StatusCode::FORBIDDEN);
        assert!(!bad.1.contains_key("set-cookie"));
        let bad = f
            .send(
                path,
                "x".repeat(65537),
                "https://identity.example",
                &f.csrf,
                None,
            )
            .await;
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
        assert!(!bad.1.contains_key("set-cookie"));
        let bad = f
            .send(
                path,
                serde_json::json!({"interaction":f.handle}).to_string(),
                "https://identity.example",
                &store::random_secret(),
                None,
            )
            .await;
        assert_eq!(bad.0, StatusCode::BAD_REQUEST);
        assert!(!bad.1.contains_key("set-cookie"));
    }
    for _ in 0..30 {
        let _ = f
            .limiter
            .check(
                &f.db,
                crate::rate_limits::Category::LoginSource,
                "127.0.0.1",
            )
            .await;
    }
    for path in [OPTIONS, FINISH] {
        let limited = f
            .send(path, "bad".into(), "https://evil.example", "bad", None)
            .await;
        assert_eq!(limited.0, StatusCode::TOO_MANY_REQUESTS);
        assert!(!limited.1.contains_key("set-cookie"));
    }
}

#[tokio::test]
async fn failed_binding_returns_unavailable_without_cookie_and_same_assertion_can_retry() {
    let mut f = Fixture::new().await;
    let body = f.assertion().await;
    sqlx::query("ALTER TABLE identity_oauth_requests ADD CONSTRAINT fail_binding CHECK (bound_session_id IS NULL)").execute(&f.db).await.unwrap();
    let failed = f
        .send(
            FINISH,
            body.to_string(),
            "https://identity.example",
            &f.csrf,
            None,
        )
        .await;
    assert_eq!(failed.0, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!failed.1.contains_key("set-cookie"));
    assert_eq!(
        failed.2,
        serde_json::json!({"error":"temporarily_unavailable"})
    );
    sqlx::query("ALTER TABLE identity_oauth_requests DROP CONSTRAINT fail_binding")
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.send(
            FINISH,
            body.to_string(),
            "https://identity.example",
            &f.csrf,
            None
        )
        .await
        .0,
        StatusCode::OK
    );
}
