use super::*;

async fn context(
    f: &Fixture,
    cookie: Option<String>,
    extra: &[(&str, &str)],
) -> (StatusCode, HeaderMap, serde_json::Value) {
    let mut request = Request::get("/oauth/session/logout-context");
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    for (key, value) in extra {
        request = request.header(*key, *value);
    }
    let response = f
        .app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    (status, headers, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn context_never_revokes_and_its_proof_can_logout_only_the_same_session() {
    let f = Fixture::new().await;
    let (status, headers, body) =
        context(&f, Some(f.cookie()), &[("x-nvbes-session-context", "1")]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-store");
    assert!(!headers.contains_key("access-control-allow-origin"));
    let active: bool =
        sqlx::query_scalar("SELECT revoked_at IS NULL FROM identity_sessions WHERE id=$1")
            .bind(f.session)
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert!(active);
    let csrf = body["session_csrf_token"].as_str().unwrap();
    assert_ne!(csrf, f.token);
    assert_eq!(
        f.post("/oauth/logout", csrf, "{}", "https://identity.example")
            .await
            .0,
        StatusCode::OK
    );
    let (_, _, after) = context(&f, Some(f.cookie()), &[("x-nvbes-session-context", "1")]).await;
    assert!(after["session_csrf_token"].is_null());
}

#[tokio::test]
async fn context_rejects_navigation_cross_origin_and_duplicated_security_headers() {
    let f = Fixture::new().await;
    for headers in [
        vec![],
        vec![
            ("x-nvbes-session-context", "1"),
            ("origin", "https://account.example"),
        ],
        vec![
            ("x-nvbes-session-context", "1"),
            ("sec-fetch-site", "cross-site"),
        ],
        vec![
            ("x-nvbes-session-context", "1"),
            ("sec-fetch-mode", "navigate"),
        ],
        vec![
            ("x-nvbes-session-context", "1"),
            ("sec-fetch-dest", "iframe"),
        ],
        vec![
            ("x-nvbes-session-context", "1"),
            ("x-nvbes-session-context", "1"),
        ],
    ] {
        assert_eq!(
            context(&f, Some(f.cookie()), &headers).await.0,
            StatusCode::FORBIDDEN
        );
    }
    let (status, _, body) = context(&f, None, &[("x-nvbes-session-context", "1")]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["session_csrf_token"].is_null());
}
