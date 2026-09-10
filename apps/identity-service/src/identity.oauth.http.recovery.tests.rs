use super::*;
use crate::{
    mfa_crypto::MfaCrypto,
    test_fixtures::{isolated_database, session},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use nvbes_core::mfa::{current_counter, generate_totp_code};
use tower::ServiceExt;
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};

const GENERATE: &str = "/oauth/session/recovery/codes/generate";
const REDEEM: &str = "/oauth/session/recovery/redeem";
const OPTIONS: &str = "/oauth/recovery/registration/options";
const FINISH: &str = "/oauth/recovery/registration/finish";
const ORIGIN: &str = "https://identity.example";

struct Fixture {
    db: PgPool,
    app: Router,
    token: String,
    browser: String,
    csrf: String,
    principal: uuid::Uuid,
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
        let crypto = MfaCrypto::with_rotation(1, [8; 32], None).unwrap();
        let pending = crate::totp::start(&db, &crypto, &token).await.unwrap();
        crate::totp::confirm(
            &db,
            &crypto,
            &token,
            pending.factor_id,
            &generate_totp_code(&pending.secret_base32, current_counter(chrono::Utc::now())),
        )
        .await
        .unwrap();
        let security = BrowserSecurity::new(ORIGIN, false).unwrap();
        let browser = security.browser_cookie().token;
        let csrf = security.session_csrf_token(&token, &browser).unwrap();
        let server = Arc::new(crate::webauthn::build_server("identity.example", ORIGIN).unwrap());
        let limiter = RateLimiter::new(rand::random()).unwrap();
        let app = router(db.clone(), security, server, limiter.clone());
        Self {
            db,
            app,
            token,
            browser,
            csrf,
            principal,
            limiter,
        }
    }
    fn cookie(&self) -> String {
        format!(
            "__Host-nvbes-browser={}; __Host-nvbes-session={}",
            self.browser, self.token
        )
    }
    async fn send(
        &self,
        path: &str,
        body: &str,
        cookie: &str,
        csrf: &str,
        origin: &str,
    ) -> (StatusCode, HeaderMap, serde_json::Value) {
        let request = Request::builder()
            .method("POST")
            .uri(path)
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:7000".parse::<std::net::SocketAddr>().unwrap(),
            ))
            .header("origin", origin)
            .header("content-type", "application/json")
            .header("cookie", cookie)
            .header("x-csrf-token", csrf)
            .body(Body::from(body.to_owned()))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        let status = response.status();
        let headers = response.headers().clone();
        let json =
            serde_json::from_slice(&to_bytes(response.into_body(), 131072).await.unwrap()).unwrap();
        (status, headers, json)
    }
    async fn codes(&self) -> Vec<String> {
        let (status, _, body) = self
            .send(GENERATE, "{}", &self.cookie(), &self.csrf, ORIGIN)
            .await;
        assert_eq!(status, StatusCode::OK);
        serde_json::from_value(body["codes"].clone()).unwrap()
    }
    async fn redeem(&self, code: &str, cookie: &str, csrf: &str) -> (String, String) {
        let (status, headers, body) = self
            .send(
                REDEEM,
                &serde_json::json!({"code":code}).to_string(),
                cookie,
                csrf,
                ORIGIN,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.get("token").is_none());
        assert_eq!(body["recovery"], true);
        let cookies = headers
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|h| h.to_str().unwrap())
            .collect::<Vec<_>>();
        let recovery = cookies
            .iter()
            .find(|v| v.starts_with("__Host-nvbes-recovery="))
            .unwrap();
        assert!(
            recovery.contains("HttpOnly")
                && recovery.contains("Secure")
                && recovery.contains("Max-Age=300")
        );
        assert!(
            cookies
                .iter()
                .any(|v| v.starts_with("__Host-nvbes-session=") && v.contains("Max-Age=0"))
        );
        (
            format!(
                "__Host-nvbes-browser={}; {}",
                self.browser,
                recovery.split(';').next().unwrap()
            ),
            body["csrf_token"].as_str().unwrap().to_owned(),
        )
    }
}

#[tokio::test]
async fn recovery_http_replaces_factor_without_issuing_an_ordinary_session() {
    let f = Fixture::new().await;
    let codes = f.codes().await;
    let (cookie, csrf) = f.redeem(&codes[0], &f.cookie(), &f.csrf).await;
    assert_eq!(
        f.send(GENERATE, "{}", &f.cookie(), &f.csrf, ORIGIN).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.send(GENERATE, "{}", &cookie, &csrf, ORIGIN).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.send(OPTIONS, "{}", &cookie, &f.csrf, ORIGIN).await.0,
        StatusCode::FORBIDDEN
    );
    let (status, _, started) = f.send(OPTIONS, "{}", &cookie, &csrf, ORIGIN).await;
    assert_eq!(status, StatusCode::OK);
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let credential = authenticator
        .do_registration(
            ORIGIN.parse().unwrap(),
            serde_json::from_value(started["options"].clone()).unwrap(),
        )
        .unwrap();
    let body=serde_json::json!({"ceremony_id":started["ceremony_id"],"credential":credential,"label":"Recovered key"}).to_string();
    let (status, headers, result) = f.send(FINISH, &body, &cookie, &csrf, ORIGIN).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["recovered"], true);
    assert_eq!(result["must_reauthenticate"], true);
    assert!(
        headers
            .get_all(header::SET_COOKIE)
            .iter()
            .all(|v| v.to_str().unwrap().contains("Max-Age=0"))
    );
    assert_eq!(
        f.send(FINISH, &body, &cookie, &csrf, ORIGIN).await.0,
        StatusCode::BAD_REQUEST
    );
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_sessions WHERE principal_id=$1 AND revoked_at IS NULL",
    )
    .bind(f.principal)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(active, 0);
}

#[tokio::test]
async fn recovery_routes_reject_origin_csrf_and_ambiguous_bodies() {
    let f = Fixture::new().await;
    let codes = f.codes().await;
    for path in [GENERATE, REDEEM] {
        assert_eq!(
            f.send(path, "{", &f.cookie(), &f.csrf, "https://evil.example")
                .await
                .0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            f.send(
                path,
                "{",
                &f.cookie(),
                &crate::oauth::store::random_secret(),
                ORIGIN
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    let (cookie, csrf) = f.redeem(&codes[0], &f.cookie(), &f.csrf).await;
    for path in [OPTIONS, FINISH] {
        assert_eq!(
            f.send(path, "{", &cookie, &csrf, "https://evil.example")
                .await
                .0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            f.send(path, "{", &cookie, &f.csrf, ORIGIN).await.0,
            StatusCode::FORBIDDEN
        );
    }
    for body in [
        "[]".to_owned(),
        "null".into(),
        "{\"unknown\":true}".into(),
        "x".repeat(4097),
    ] {
        assert_eq!(
            f.send(OPTIONS, &body, &cookie, &csrf, ORIGIN).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let id = uuid::Uuid::new_v4();
    for body in [
        format!(r#"{{"ceremony_id":"{id}","ceremony_id":"{id}","credential":{{}},"label":"Key"}}"#),
        "[]".into(),
        "x".repeat(65537),
    ] {
        assert_eq!(
            f.send(FINISH, &body, &cookie, &csrf, ORIGIN).await.0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn malformed_redemptions_consume_shared_account_quota_and_return_retry_after() {
    let f = Fixture::new().await;
    for body in [
        "[]",
        "null",
        "{",
        "{\"code\":\"bad\",\"code\":\"bad\"}",
        "{\"code\":\"bad\",\"extra\":true}",
    ] {
        assert_eq!(
            f.send(REDEEM, body, &f.cookie(), &f.csrf, ORIGIN).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let (status, headers, _) = f.send(GENERATE, "{}", &f.cookie(), &f.csrf, ORIGIN).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(headers["retry-after"], "600");
    assert!(headers.get(header::SET_COOKIE).is_none());
}

#[tokio::test]
async fn source_limits_and_store_failure_fail_closed_before_cookie_changes() {
    let f = Fixture::new().await;
    for _ in 0..30 {
        f.limiter
            .check(&f.db, Category::LoginSource, "127.0.0.1")
            .await
            .unwrap();
    }
    for path in [GENERATE, REDEEM, OPTIONS, FINISH] {
        assert_eq!(
            f.send(path, "{", "", "bad", "https://evil.example").await.0,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    sqlx::query("ALTER TABLE identity_rate_buckets RENAME TO unavailable_buckets")
        .execute(&f.db)
        .await
        .unwrap();
    let (status, headers, _) = f.send(REDEEM, "{}", &f.cookie(), &f.csrf, ORIGIN).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(headers.get(header::SET_COOKIE).is_none());
}

#[tokio::test]
async fn replacing_recovery_session_does_not_reset_ceremony_quota() {
    let f = Fixture::new().await;
    let codes = f.codes().await;
    let (cookie, csrf) = f.redeem(&codes[0], &f.cookie(), &f.csrf).await;
    for _ in 0..20 {
        f.limiter
            .check(&f.db, Category::WebauthnAccount, &f.principal.to_string())
            .await
            .unwrap();
    }
    assert_eq!(
        f.send(OPTIONS, "{}", &cookie, &csrf, ORIGIN).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    let (id, token) = session(&f.db).await;
    sqlx::query("UPDATE identity_sessions SET principal_id=$1 WHERE id=$2")
        .bind(f.principal)
        .bind(id)
        .execute(&f.db)
        .await
        .unwrap();
    let security = BrowserSecurity::new(ORIGIN, false).unwrap();
    let normal = format!(
        "__Host-nvbes-browser={}; __Host-nvbes-session={token}",
        f.browser
    );
    let proof = security.session_csrf_token(&token, &f.browser).unwrap();
    let (cookie, csrf) = f.redeem(&codes[1], &normal, &proof).await;
    assert_eq!(
        f.send(OPTIONS, "{}", &cookie, &csrf, ORIGIN).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn failed_redemption_does_not_issue_recovery_or_clear_the_valid_session() {
    let f = Fixture::new().await;
    let codes = f.codes().await;
    sqlx::query("ALTER TABLE identity_outbox ADD CONSTRAINT injected_http_recovery_failure CHECK(event_type<>'identity.mfa_recovery_started')").execute(&f.db).await.unwrap();
    let body = serde_json::json!({"code":codes[0]}).to_string();
    let (status, headers, _) = f.send(REDEEM, &body, &f.cookie(), &f.csrf, ORIGIN).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(headers.get(header::SET_COOKIE).is_none());
    assert!(crate::totp::management::list(&f.db, &f.token).await.is_ok());
    sqlx::query("ALTER TABLE identity_outbox DROP CONSTRAINT injected_http_recovery_failure")
        .execute(&f.db)
        .await
        .unwrap();
    f.redeem(&codes[0], &f.cookie(), &f.csrf).await;
}
