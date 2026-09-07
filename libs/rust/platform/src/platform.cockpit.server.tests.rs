use super::*;
use axum::{
    body::Body,
    http::{Request, header},
};
use tower::ServiceExt;

pub(crate) fn token(role: &str, amr: &[&str], auth_time: i64) -> String {
    let claims = json!({"sub":"operator-one","role":role,"amr":amr,"auth_time":auth_time,
        "iss":"test-identity","aud":"platform-operations","exp":chrono::Utc::now().timestamp()+60});
    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(b"test-signing-key-for-platform-operations"),
    )
    .unwrap()
}

fn setup() -> Router {
    create_platform_cockpit_router(PlatformCockpitState {
        environment: "test".into(),
        auth_policy: Arc::new(OperatorAuthPolicy::test_policy()),
        db: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        context: Arc::new(ContextClient::new(vec![], false).unwrap()),
    })
}

#[tokio::test]
async fn public_liveness_and_protected_overview() {
    let response = setup()
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = setup()
        .oneshot(
            Request::builder()
                .uri("/api/v1/overview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn role_and_mfa_are_verified_not_self_asserted_headers() {
    for (role, amr) in [
        ("support_agent", vec!["mfa"]),
        ("platform_owner", vec!["pwd"]),
    ] {
        let response = setup()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/overview")
                    .header(
                        header::AUTHORIZATION,
                        format!(
                            "Bearer {}",
                            token(role, &amr, chrono::Utc::now().timestamp())
                        ),
                    )
                    .header("x-nvbes-mfa-step-up", "true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
async fn missing_context_is_unknown_and_domain_actions_fail_closed() {
    let bearer = format!(
        "Bearer {}",
        token("platform_owner", &["totp"], chrono::Utc::now().timestamp())
    );
    let response = setup()
        .oneshot(
            Request::builder()
                .uri("/api/v1/overview")
                .header(header::AUTHORIZATION, &bearer)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 16000)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["operator"], "operator-one");
    assert_eq!(value["backup_restore"], "not_verified");
    assert!(
        value["services"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["status"] == "not_configured")
    );
    let response = setup()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/actions")
                .header(header::AUTHORIZATION, bearer)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}

#[test]
fn stale_signed_authentication_does_not_become_fresh_with_header() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        format!(
            "Bearer {}",
            token(
                "platform_owner",
                &["mfa"],
                chrono::Utc::now().timestamp() - 600
            )
        )
        .parse()
        .unwrap(),
    );
    headers.insert("x-nvbes-mfa-step-up", "true".parse().unwrap());
    let actor = OperatorAuthPolicy::test_policy()
        .authenticate_headers(&headers)
        .unwrap();
    assert!(!actor.has_mfa_step_up);
}
