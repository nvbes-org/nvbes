use super::*;
use crate::{
    oauth::clients::ClientRegistry,
    tokens::{TokenService, tests::config},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};

const RETURN: &str = "https://account.example/oauth/logout/callback";

fn registry(allow_return: bool) -> ClientRegistry {
    ClientRegistry::from_json(&json!([{
        "client_id":"account-web","display_name":"Account",
        "redirect_uris":["https://account.example/callback"],
        "post_logout_redirect_uris":if allow_return {vec![RETURN]} else {vec![]},
        "resources":{"https://api.example/account":{"audience":"nvbes-account-service","scopes":["account:read"]}},
        "allow_refresh":false,"require_dpop":false
    }]).to_string(), false).unwrap()
}

fn mount(f: &mut Fixture, allow_return: bool) {
    f.app = crate::oauth::http::rp_logout_router(
        f.db.clone(),
        Arc::new(registry(allow_return)),
        Arc::new(TokenService::new(config()).unwrap()),
        BrowserSecurity::new("https://identity.example", false).unwrap(),
        crate::rate_limits::RateLimiter::new([53; 32]).unwrap(),
    )
    .layer(axum::Extension(axum::extract::ConnectInfo(
        "127.0.0.1:4000".parse::<std::net::SocketAddr>().unwrap(),
    )));
}

async fn fields(f: &Fixture) -> String {
    let principal: uuid::Uuid =
        sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
            .bind(f.session)
            .fetch_one(&f.db)
            .await
            .unwrap();
    let config = config();
    let now = chrono::Utc::now().timestamp() as u64;
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(config.key_id);
    let token = encode(&header, &json!({"iss":config.issuer,"aud":"account-web","sub":principal,"sid":f.session,
        "iat":now-1000,"exp":now-100,"auth_time":now-1100,"amr":["pwd"],"nonce":"nonce","at_hash":"hash"}),
        &EncodingKey::from_rsa_pem(config.private_key_pem.as_bytes()).unwrap()).unwrap();
    let mut url = reqwest::Url::parse("https://identity.example/").unwrap();
    url.query_pairs_mut()
        .append_pair("id_token_hint", &token)
        .append_pair("client_id", "account-web")
        .append_pair("post_logout_redirect_uri", RETURN)
        .append_pair("state", "opaque+&=state");
    url.query().unwrap().to_string()
}

fn csrf(f: &Fixture) -> String {
    BrowserSecurity::new("https://identity.example", false)
        .unwrap()
        .session_csrf_token(&f.token, &f.browser)
        .unwrap()
}

async fn start(f: &Fixture, fields: &str, post: bool) -> (StatusCode, HeaderMap) {
    let request = if post {
        Request::post("/oauth/end-session")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("origin", "https://account.example")
            .body(Body::from(fields.to_owned()))
            .unwrap()
    } else {
        Request::get(format!("/oauth/end-session?{fields}"))
            .header("cookie", f.cookie())
            .body(Body::empty())
            .unwrap()
    };
    let response = f.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    (response.status(), response.headers().clone())
}

fn body(headers: &HeaderMap) -> String {
    let location = headers["location"].to_str().unwrap();
    assert!(location.starts_with("/logout#request="));
    json!({"request":location.strip_prefix("/logout#request=").unwrap()}).to_string()
}

async fn revoked(f: &Fixture) -> bool {
    sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity_sessions WHERE id=$1")
        .bind(f.session)
        .fetch_one(&f.db)
        .await
        .unwrap()
}

#[tokio::test]
async fn get_and_cross_site_post_prepare_without_revocation_then_confirm_atomically_once() {
    for post in [false, true] {
        let mut f = Fixture::new().await;
        mount(&mut f, true);
        let (status, headers) = start(&f, &fields(&f).await, post).await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        let body = body(&headers);
        let csrf = csrf(&f);
        assert!(!revoked(&f).await);
        assert_eq!(
            f.post(
                "/oauth/logout/prepare",
                &csrf,
                &body,
                "https://identity.example"
            )
            .await
            .2["prepared"],
            true
        );
        assert!(!revoked(&f).await);
        let (first, second) = tokio::join!(
            f.post(
                "/oauth/logout/confirm",
                &csrf,
                &body,
                "https://identity.example"
            ),
            f.post(
                "/oauth/logout/confirm",
                &csrf,
                &body,
                "https://identity.example"
            )
        );
        for (status, headers, result) in [first, second] {
            assert_eq!(status, StatusCode::OK);
            assert_eq!(result["logged_out"], true);
            assert_eq!(
                result["redirect_uri"],
                format!("{RETURN}?state=opaque%2B%26%3Dstate")
            );
            assert!(
                headers["set-cookie"]
                    .to_str()
                    .unwrap()
                    .contains("Max-Age=0")
            );
        }
        assert!(revoked(&f).await);
        let events: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_audit_events WHERE event_type='identity.session.logged_out'")
            .fetch_one(&f.db).await.unwrap();
        assert_eq!(events, 1);
    }
}

#[tokio::test]
async fn wrong_origin_csrf_session_and_changed_registration_never_revoke_or_redirect() {
    let mut f = Fixture::new().await;
    mount(&mut f, true);
    let (_, headers) = start(&f, &fields(&f).await, false).await;
    let body = body(&headers);
    let original_csrf = csrf(&f);
    for path in ["/oauth/logout/prepare", "/oauth/logout/confirm"] {
        assert_eq!(
            f.post(path, &original_csrf, &body, "https://attacker.example")
                .await
                .0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            f.post(path, &random_secret(), "{", "https://identity.example")
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    let original = f.token.clone();
    let (_, other) = session(&f.db).await;
    f.token = other;
    for path in ["/oauth/logout/prepare", "/oauth/logout/confirm"] {
        let (status, headers, _) = f
            .post(path, &csrf(&f), &body, "https://identity.example")
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!headers.contains_key("location"));
        assert!(!headers.contains_key("set-cookie"));
    }
    f.token = original;
    mount(&mut f, false);
    assert_eq!(
        f.post(
            "/oauth/logout/confirm",
            &csrf(&f),
            &body,
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert!(!revoked(&f).await);
}

#[tokio::test]
async fn invalid_preparation_and_store_failure_never_issue_a_return_or_clear_cookies() {
    let mut f = Fixture::new().await;
    mount(&mut f, true);
    for fields in [
        "post_logout_redirect_uri=https%3A%2F%2Fattacker.example",
        "client_id=account-web&client_id=account-web",
        "id_token_hint=invalid",
    ] {
        for post in [false, true] {
            let (status, headers) = start(&f, fields, post).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert!(!headers.contains_key("location"));
        }
    }
    let (_, headers) = start(&f, &fields(&f).await, true).await;
    let body = body(&headers);
    let csrf = csrf(&f);
    f.db.close().await;
    for path in ["/oauth/logout/prepare", "/oauth/logout/confirm"] {
        let (status, headers, result) =
            f.post(path, &csrf, &body, "https://identity.example").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            result["error"],
            Value::String("temporarily_unavailable".into())
        );
        assert!(!headers.contains_key("set-cookie"));
    }
}

#[tokio::test]
async fn audit_failure_rolls_back_revocation_and_never_returns_a_destination() {
    let mut f = Fixture::new().await;
    mount(&mut f, true);
    let (_, headers) = start(&f, &fields(&f).await, true).await;
    let body = body(&headers);
    sqlx::query("CREATE FUNCTION reject_logout_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic audit failure'; END $$")
        .execute(&f.db).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_logout_audit BEFORE INSERT ON identity_audit_events FOR EACH ROW WHEN (NEW.event_type='identity.session.logged_out') EXECUTE FUNCTION reject_logout_audit()")
        .execute(&f.db).await.unwrap();
    let (status, headers, result) = f
        .post(
            "/oauth/logout/confirm",
            &csrf(&f),
            &body,
            "https://identity.example",
        )
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!headers.contains_key("set-cookie"));
    assert!(result.get("redirect_uri").is_none());
    assert!(!revoked(&f).await);
}
