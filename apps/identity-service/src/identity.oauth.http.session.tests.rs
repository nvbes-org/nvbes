use crate::{
    browser::BrowserSecurity,
    mfa_crypto::MfaCrypto,
    oauth::{http::authorization_router, store::random_secret},
    test_fixtures::{self, clients, database, session},
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode},
};
use nvbes_core::mfa::{current_counter, generate_totp_code, generate_totp_secret};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

#[path = "identity.oauth.http.authorization.tests.rs"]
mod authorization_tests;
#[path = "identity.oauth.http.limits.tests.rs"]
mod limits_tests;
#[path = "identity.oauth.http.login.tests.rs"]
mod login_tests;
#[path = "identity.oauth.http.navigation.tests.rs"]
mod navigation_tests;
#[path = "identity.oauth.http.totp.tests.rs"]
mod totp_tests;

struct Fixture {
    db: PgPool,
    app: Router,
    session: uuid::Uuid,
    token: String,
    browser: String,
    crypto: Arc<MfaCrypto>,
}

impl Fixture {
    async fn new() -> Self {
        let db = database().await;
        // The runtime rotation test intentionally visits every factor. Keep
        // these HTTP fixtures in a private schema with their own encryption key.
        let schema = format!("identity_http_{}", uuid::Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE SCHEMA {schema}"))
            .execute(&db)
            .await
            .unwrap();
        let options = db.connect_options().as_ref().clone();
        db.close().await;
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let schema = schema.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(schema)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(options)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&db).await.unwrap();
        let (session, token) = session(&db).await;
        let crypto = Arc::new(MfaCrypto::with_rotation(1, [7; 32], None).unwrap());
        let app = authorization_router(
            db.clone(),
            Arc::new(clients()),
            BrowserSecurity::new("https://identity.example", false).unwrap(),
            crypto.clone(),
            crate::rate_limits::RateLimiter::new([53; 32]).unwrap(),
        )
        .layer(axum::Extension(axum::extract::ConnectInfo(
            "127.0.0.1:4000".parse::<std::net::SocketAddr>().unwrap(),
        )));
        Self {
            db,
            app,
            session,
            token,
            browser: random_secret(),
            crypto,
        }
    }

    fn cookie(&self) -> String {
        format!(
            "__Host-nvbes-browser={}; __Host-nvbes-session={}",
            self.browser, self.token
        )
    }

    async fn authorize(&self) -> (HeaderMap, serde_json::Value) {
        let mut url = reqwest::Url::parse("https://identity.example/oauth/authorize").unwrap();
        let input = test_fixtures::input();
        for (key, value) in [
            ("client_id", input.client_id),
            ("redirect_uri", input.redirect_uri),
            ("response_type", input.response_type),
            ("scope", input.scope),
            ("resource", input.resource),
            ("state", input.state),
            ("nonce", input.nonce),
            ("code_challenge", input.code_challenge),
            ("code_challenge_method", input.code_challenge_method),
        ] {
            url.query_pairs_mut().append_pair(key, &value);
        }
        let request = Request::get(format!("{}?{}", url.path(), url.query().unwrap()))
            .header("cookie", self.cookie())
            .body(Body::empty())
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), 8192).await.unwrap();
        (headers, serde_json::from_slice(&body).unwrap())
    }

    async fn post(
        &self,
        path: &str,
        csrf: &str,
        body: &str,
        origin: &str,
    ) -> (StatusCode, HeaderMap, serde_json::Value) {
        let request = Request::post(path)
            .header("origin", origin)
            .header("cookie", self.cookie())
            .header("x-csrf-token", csrf)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
        (status, headers, serde_json::from_slice(&bytes).unwrap())
    }

    async fn active_factor(&self) -> (uuid::Uuid, String) {
        let principal: uuid::Uuid =
            sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
                .bind(self.session)
                .fetch_one(&self.db)
                .await
                .unwrap();
        let factor = uuid::Uuid::new_v4();
        let secret = generate_totp_secret();
        let sealed = self.crypto.seal(factor, &secret).unwrap();
        sqlx::query("INSERT INTO identity_auth_factors(id,principal_id,kind,state,secret_ciphertext,secret_nonce,key_version) VALUES($1,$2,'totp','active',$3,$4,$5)")
            .bind(factor).bind(principal).bind(sealed.ciphertext).bind(sealed.nonce.as_slice())
            .bind(sealed.key_version).execute(&self.db).await.unwrap();
        let code = generate_totp_code(&secret, current_counter(chrono::Utc::now()));
        (factor, code)
    }
}

#[tokio::test]
async fn csrf_from_authorization_is_stable_across_tabs_and_logout_is_audited_once() {
    let f = Fixture::new().await;
    let (headers, first) = f.authorize().await;
    assert!(headers["set-cookie"].to_str().unwrap().contains(&f.browser));
    let (_, second) = f.authorize().await;
    assert_ne!(first["interaction"], second["interaction"]);
    assert_ne!(first["csrf_token"], second["csrf_token"]);
    assert_eq!(first["session_csrf_token"], second["session_csrf_token"]);
    let csrf = first["session_csrf_token"].as_str().unwrap();
    let (other, _) = session(&f.db).await;
    for _ in 0..2 {
        let (status, headers, body) = f
            .post("/oauth/logout", csrf, "{}", "https://identity.example")
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["logged_out"], true);
        assert!(
            headers["set-cookie"]
                .to_str()
                .unwrap()
                .contains("Max-Age=0")
        );
    }
    let state: (bool, i64) = sqlx::query_as("SELECT s.revoked_at IS NOT NULL,(SELECT count(*) FROM identity_audit_events a WHERE a.principal_id=s.principal_id AND event_type='identity.session.logged_out') FROM identity_sessions s WHERE id=$1")
        .bind(f.session).fetch_one(&f.db).await.unwrap();
    assert_eq!(state, (true, 1));
    let other_active: bool =
        sqlx::query_scalar("SELECT revoked_at IS NULL FROM identity_sessions WHERE id=$1")
            .bind(other)
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert!(other_active);
}

#[tokio::test]
async fn session_routes_reject_wrong_csrf_before_json_and_do_not_consume_totp() {
    let f = Fixture::new().await;
    let (_, authorization) = f.authorize().await;
    let (factor, code) = f.active_factor().await;
    let body = serde_json::json!({"code": code}).to_string();
    for path in ["/oauth/logout", "/oauth/session/step-up/totp"] {
        for csrf in [
            random_secret(),
            authorization["csrf_token"].as_str().unwrap().to_owned(),
        ] {
            assert_eq!(
                f.post(path, &csrf, "{", "https://identity.example").await.0,
                StatusCode::FORBIDDEN
            );
        }
        let csrf = authorization["session_csrf_token"].as_str().unwrap();
        assert_eq!(
            f.post(path, csrf, &body, "https://account.example").await.0,
            StatusCode::FORBIDDEN
        );
    }
    let csrf = authorization["session_csrf_token"].as_str().unwrap();
    assert_eq!(
        f.post(
            "/oauth/session/step-up/totp",
            csrf,
            &body,
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        f.post(
            "/oauth/session/step-up/totp",
            csrf,
            &body,
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let consumed: bool = sqlx::query_scalar(
        "SELECT last_accepted_counter IS NOT NULL FROM identity_auth_factors WHERE id=$1",
    )
    .bind(factor)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert!(consumed);
}

#[tokio::test]
async fn csrf_cannot_be_moved_to_another_session_and_suspended_accounts_cannot_step_up() {
    let mut f = Fixture::new().await;
    let (_, authorization) = f.authorize().await;
    let csrf = authorization["session_csrf_token"].as_str().unwrap();
    let original = f.token.clone();
    let (_, other_token) = session(&f.db).await;
    f.token = other_token;
    assert_eq!(
        f.post("/oauth/logout", csrf, "{}", "https://identity.example")
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    f.token = original;
    let (factor, code) = f.active_factor().await;
    sqlx::query("UPDATE identity_principals SET status='suspended' WHERE id=(SELECT principal_id FROM identity_sessions WHERE id=$1)")
        .bind(f.session).execute(&f.db).await.unwrap();
    let body = serde_json::json!({"code": code}).to_string();
    assert_eq!(
        f.post(
            "/oauth/session/step-up/totp",
            csrf,
            &body,
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let untouched: bool = sqlx::query_scalar(
        "SELECT last_accepted_counter IS NULL FROM identity_auth_factors WHERE id=$1",
    )
    .bind(factor)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert!(untouched);
}

#[tokio::test]
async fn all_hosted_mutations_reject_cross_origin_before_body_parsing() {
    let f = Fixture::new().await;
    for path in [
        "/oauth/authorize/login",
        "/oauth/authorize/approve",
        "/oauth/authorize/deny",
        "/oauth/logout",
        "/oauth/session/step-up/totp",
    ] {
        assert_eq!(
            f.post(path, &random_secret(), "{", "https://account.example")
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    // GET authorization remains reachable through a top-level OAuth navigation.
    f.authorize().await;
}

#[tokio::test]
async fn unavailable_session_store_never_reports_success_or_clears_cookie() {
    let f = Fixture::new().await;
    let (_, authorization) = f.authorize().await;
    let csrf = authorization["session_csrf_token"].as_str().unwrap();
    f.db.close().await;
    for path in ["/oauth/logout", "/oauth/session/step-up/totp"] {
        let (status, headers, body) = f
            .post(
                path,
                csrf,
                r#"{"code":"123456"}"#,
                "https://identity.example",
            )
            .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["error"], "temporarily_unavailable");
        assert!(!headers.contains_key("set-cookie"));
    }
}

#[tokio::test]
async fn hosted_login_returns_csrf_for_the_new_session_cookie() {
    let mut f = Fixture::new().await;
    let email = format!("session-http-{}@example.invalid", uuid::Uuid::new_v4());
    let password = "Hosted-login-test-password!";
    crate::auth::create_synthetic_identity(&f.db, &email, password)
        .await
        .unwrap();
    let (_, authorization) = f.authorize().await;
    let body = serde_json::json!({
        "interaction": authorization["interaction"], "email": email, "password": password,
    })
    .to_string();
    let (status, headers, login) = f
        .post(
            "/oauth/authorize/login",
            authorization["csrf_token"].as_str().unwrap(),
            &body,
            "https://identity.example",
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let cookie = headers["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    let new_session = cookie.strip_prefix("__Host-nvbes-session=").unwrap();
    assert_ne!(new_session, f.token);
    f.token = new_session.to_owned();
    assert_ne!(
        login["session_csrf_token"],
        authorization["session_csrf_token"]
    );
    assert_ne!(login["session_csrf_token"], login["csrf_token"]);
    assert_eq!(
        f.post(
            "/oauth/logout",
            authorization["session_csrf_token"].as_str().unwrap(),
            "{}",
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.post(
            "/oauth/logout",
            login["session_csrf_token"].as_str().unwrap(),
            "{}",
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::OK
    );
}
