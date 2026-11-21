use super::*;
use crate::{
    oauth::{codes, store::RequestKind},
    test_fixtures as fixtures,
};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use serde_json::{Value, json};
use tower::ServiceExt;

struct Fixture {
    db: PgPool,
    app: Router,
    token: String,
    session: uuid::Uuid,
    grant: uuid::Uuid,
}
impl Fixture {
    async fn new() -> Self {
        let db = fixtures::isolated_database().await;
        let clients = Arc::new(fixtures::clients());
        let tokens = Arc::new(TokenService::new(crate::tokens::tests::config()).unwrap());
        let (session, session_token) = fixtures::session(&db).await;
        let handle = fixtures::request(&db, &clients, RequestKind::Authorization).await;
        let code = fixtures::authorize(&db, &clients, &handle, &session_token)
            .await
            .unwrap();
        let grant = codes::exchange(&db, &clients, fixtures::exchange(&code.code))
            .await
            .unwrap();
        let token = tokens
            .issue_grant(&db, &clients, &grant)
            .await
            .unwrap()
            .access_token;
        let resources=ResourceServers::from_json(&json!([
            {"client_id":"account-api","audience":"nvbes-account-service","secret":URL_SAFE_NO_PAD.encode([31;32])},
            {"client_id":"billing-api","audience":"nvbes-billing-service","secret":URL_SAFE_NO_PAD.encode([32;32])}
        ]).to_string()).unwrap();
        let app = router(
            db.clone(),
            clients,
            tokens,
            Arc::new(resources),
            RateLimiter::new(rand::random()).unwrap(),
        );
        Self {
            db,
            app,
            token,
            session,
            grant: grant.id,
        }
    }
    async fn request(
        &self,
        credentials: Option<(&str, u8)>,
        body: String,
        suffix: &str,
    ) -> (StatusCode, Value) {
        let mut request = Request::post(format!("/oauth/introspect{suffix}"))
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:8000".parse::<std::net::SocketAddr>().unwrap(),
            ))
            .header("content-type", "application/x-www-form-urlencoded");
        if let Some((id, byte)) = credentials {
            request = request.header(
                "authorization",
                format!(
                    "Basic {}",
                    STANDARD.encode(format!("{id}:{}", URL_SAFE_NO_PAD.encode([byte; 32])))
                ),
            );
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 32_768).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
}

#[tokio::test]
async fn authenticated_resource_observes_live_session_and_grant_revocation() {
    let f = Fixture::new().await;
    let form = format!("token={}", f.token);
    let (status, value) = f.request(Some(("account-api", 31)), form.clone(), "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["active"], true);
    assert_eq!(value["aud"], "nvbes-account-service");
    assert!(value.get("cnf").is_none());
    assert_eq!(value["sid"], f.session.to_string());
    let other = f.request(Some(("billing-api", 32)), form.clone(), "").await;
    assert_eq!(other, (StatusCode::OK, json!({"active":false})));
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(f.session)
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.request(Some(("account-api", 31)), form, "").await,
        (StatusCode::OK, json!({"active":false}))
    );
    let f = Fixture::new().await;
    sqlx::query("UPDATE identity_oauth_grants SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(f.grant)
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.request(Some(("account-api", 31)), format!("token={}", f.token), "")
            .await,
        (StatusCode::OK, json!({"active":false}))
    );
}

#[tokio::test]
async fn introspection_authentication_and_form_boundaries_are_enforced() {
    let f = Fixture::new().await;
    for credential in [None, Some(("account-api", 32)), Some(("unknown-api", 31))] {
        assert_eq!(
            f.request(credential, "token=unknown".into(), "").await,
            (StatusCode::UNAUTHORIZED, json!({"error":"invalid_client"}))
        );
    }
    for body in [
        "",
        "token=a&token=b",
        "token=a&audience=nvbes-billing-service",
        "token=a&token_type_hint=x&token_type_hint=y",
    ] {
        assert_eq!(
            f.request(Some(("account-api", 31)), body.into(), "")
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        f.request(Some(("account-api", 31)), "token=a".into(), "?token=b")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.request(
            Some(("account-api", 31)),
            "token=unknown&token_type_hint=unknown".into(),
            ""
        )
        .await,
        (StatusCode::OK, json!({"active":false}))
    );
}

#[tokio::test]
async fn unavailable_grant_store_returns_service_error_not_active_or_inactive() {
    let f = Fixture::new().await;
    sqlx::query("ALTER TABLE identity_oauth_grants RENAME TO unavailable_grants")
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.request(Some(("account-api", 31)), format!("token={}", f.token), "")
            .await,
        (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({"error":"temporarily_unavailable"})
        )
    );
}

#[tokio::test]
async fn a_blocked_session_lookup_times_out_without_claiming_activity() {
    let f = Fixture::new().await;
    let mut lock = f.db.begin().await.unwrap();
    sqlx::query("SELECT id FROM identity_sessions WHERE id=$1 FOR UPDATE")
        .bind(f.session)
        .execute(&mut *lock)
        .await
        .unwrap();
    let response = tokio::time::timeout(
        Duration::from_secs(5),
        f.request(Some(("account-api", 31)), format!("token={}", f.token), ""),
    )
    .await
    .unwrap();
    assert_eq!(
        response,
        (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({"error":"temporarily_unavailable"})
        )
    );
    lock.rollback().await.unwrap();
    let response = f
        .request(Some(("account-api", 31)), format!("token={}", f.token), "")
        .await;
    assert_eq!(response.0, StatusCode::OK);
    assert_eq!(response.1["active"], true);
}
