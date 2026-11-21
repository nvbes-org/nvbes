use super::*;
use crate::recovery_test_fixture::{INITIAL, fixture};
use axum::body::{Body, to_bytes};
use tower::ServiceExt;

struct Fixture {
    db: PgPool,
    app: Router,
    crypto: Arc<MfaCrypto>,
    email: String,
    cookie: String,
    csrf: String,
}
impl Fixture {
    async fn new() -> Self {
        let (db, _, email) = fixture().await;
        let crypto = Arc::new(MfaCrypto::with_rotation(1, [67; 32], None).unwrap());
        let app = router(
            db.clone(),
            BrowserSecurity::new("https://identity.example", false).unwrap(),
            RateLimiter::new([68; 32]).unwrap(),
            crypto.clone(),
            "https://identity.example/password-recovery".into(),
        )
        .layer(axum::Extension(ConnectInfo(
            "127.0.0.1:12345".parse::<SocketAddr>().unwrap(),
        )));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/oauth/password-recovery/context")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let (_, value) = json(response).await;
        let csrf = value["csrf_token"].as_str().unwrap().to_owned();
        Self {
            db,
            app,
            crypto,
            email,
            cookie,
            csrf,
        }
    }
    async fn post(&self, path: &str, body: serde_json::Value) -> Response {
        self.raw(
            path,
            &body.to_string(),
            "https://identity.example",
            &self.csrf,
        )
        .await
    }
    async fn raw(&self, path: &str, body: &str, origin: &str, csrf: &str) -> Response {
        self.app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header(header::ORIGIN, origin)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &self.cookie)
                    .header("x-csrf-token", csrf)
                    .body(Body::from(body.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }
}
async fn json(response: Response) -> (StatusCode, serde_json::Value) {
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn eligible_unknown_and_inactive_emails_share_response_and_separate_login_quota() {
    let test = Fixture::new().await;
    let expected = (StatusCode::ACCEPTED, serde_json::json!({"accepted":true}));
    for _ in 0..3 {
        assert_eq!(
            json(
                test.post(
                    "/oauth/password-recovery/request",
                    serde_json::json!({"email":test.email})
                )
                .await
            )
            .await,
            expected
        );
        assert_eq!(
            json(
                test.post(
                    "/oauth/password-recovery/request",
                    serde_json::json!({"email":"unknown@example.invalid"})
                )
                .await
            )
            .await,
            expected
        );
    }
    for email in [&test.email, "unknown@example.invalid"] {
        assert_eq!(
            test.post(
                "/oauth/password-recovery/request",
                serde_json::json!({"email":email})
            )
            .await
            .status(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    auth::authenticate(&test.db, &test.email, INITIAL)
        .await
        .unwrap();
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_recovery_deliveries")
        .fetch_one(&test.db)
        .await
        .unwrap();
    assert_eq!(rows, 3);
    let inactive = "suspended@example.invalid";
    let principal = auth::create_synthetic_identity(&test.db, inactive, INITIAL)
        .await
        .unwrap();
    sqlx::query("UPDATE identity_principals SET status='suspended' WHERE id=$1")
        .bind(principal)
        .execute(&test.db)
        .await
        .unwrap();
    assert_eq!(
        json(
            test.post(
                "/oauth/password-recovery/request",
                serde_json::json!({"email":inactive})
            )
            .await
        )
        .await,
        expected
    );
    test.db.close().await;
}

#[tokio::test]
async fn browser_binding_and_strict_body_validation_precede_delivery_mutations() {
    let test = Fixture::new().await;
    let body = serde_json::json!({"email":test.email}).to_string();
    assert_eq!(
        test.raw(
            "/oauth/password-recovery/request",
            &body,
            "https://evil.example",
            &test.csrf
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        test.raw(
            "/oauth/password-recovery/request",
            &body,
            "https://identity.example",
            &auth::random_token()
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    for body in [
        "{}",
        r#"{"email":"a@example.invalid","email":"b@example.invalid"}"#,
        r#"{"email":"a@example.invalid","extra":true}"#,
    ] {
        assert_eq!(
            test.raw(
                "/oauth/password-recovery/request",
                body,
                "https://identity.example",
                &test.csrf
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_recovery_challenges")
        .fetch_one(&test.db)
        .await
        .unwrap();
    assert_eq!(pending, 0);
    test.db.close().await;
}

#[tokio::test]
async fn http_reset_revokes_sessions_and_consumes_the_link_once() {
    let test = Fixture::new().await;
    auth::authenticate(&test.db, &test.email, INITIAL)
        .await
        .unwrap();
    assert_eq!(
        test.post(
            "/oauth/password-recovery/request",
            serde_json::json!({"email":test.email})
        )
        .await
        .status(),
        StatusCode::ACCEPTED
    );
    let (id,principal,cipher,nonce,version):(uuid::Uuid,uuid::Uuid,Vec<u8>,Vec<u8>,i16)=sqlx::query_as("SELECT challenge_id,principal_id,ciphertext,nonce,key_version FROM identity_recovery_deliveries").fetch_one(&test.db).await.unwrap();
    let plaintext = test
        .crypto
        .open_recovery(principal, id, version, &cipher, &nonce)
        .unwrap();
    let command: nvbes_email::EmailCommand = serde_json::from_str(&plaintext).unwrap();
    let nvbes_email::EmailTemplate::PasswordResetV1 { reset_url, .. } = command.template else {
        panic!("wrong template")
    };
    let link = reqwest::Url::parse(&reset_url).unwrap();
    assert!(link.query().is_none());
    let token = link.fragment().unwrap().strip_prefix("token=").unwrap();
    let body = serde_json::json!({"token":token,"password":"HTTP-recovered-password!"});
    let response = test
        .post("/oauth/password-recovery/reset", body.clone())
        .await;
    assert!(
        response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert_eq!(
        json(response).await,
        (
            StatusCode::OK,
            serde_json::json!({"reset":true,"must_reauthenticate":true})
        )
    );
    let replay = test.post("/oauth/password-recovery/reset", body).await;
    assert_eq!(replay.status(), StatusCode::BAD_REQUEST);
    assert!(!replay.headers().contains_key(header::SET_COOKIE));
    let live: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity_sessions WHERE revoked_at IS NULL")
            .fetch_one(&test.db)
            .await
            .unwrap();
    assert_eq!(live, 0);
    assert!(
        auth::authenticate(&test.db, &test.email, INITIAL)
            .await
            .is_err()
    );
    auth::authenticate(&test.db, &test.email, "HTTP-recovered-password!")
        .await
        .unwrap();
    test.db.close().await;
}

#[tokio::test]
async fn store_failure_is_unavailable_instead_of_an_accepted_request() {
    let test = Fixture::new().await;
    test.db.close().await;
    assert_eq!(
        test.post(
            "/oauth/password-recovery/request",
            serde_json::json!({"email":test.email})
        )
        .await
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}
