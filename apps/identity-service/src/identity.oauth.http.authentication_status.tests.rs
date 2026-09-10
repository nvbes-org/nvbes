use super::*;

fn configure(f: &mut Fixture, policy: &str) {
    let mut client = serde_json::to_value(clients().get("account-web").unwrap()).unwrap();
    client["minimum_authentication"] = serde_json::json!(policy);
    let registry = crate::oauth::clients::ClientRegistry::from_json(
        &serde_json::json!([client]).to_string(),
        false,
    )
    .unwrap();
    f.app = authorization_router(
        f.db.clone(),
        Arc::new(registry),
        BrowserSecurity::new("https://identity.example", false).unwrap(),
        f.crypto.clone(),
        crate::rate_limits::RateLimiter::new([53; 32]).unwrap(),
    )
    .layer(axum::Extension(axum::extract::ConnectInfo(
        "127.0.0.1:4000".parse::<std::net::SocketAddr>().unwrap(),
    )));
}

#[tokio::test]
async fn status_tracks_bound_session_current_policy_and_real_step_up_without_granting_consent() {
    let mut f = Fixture::new().await;
    configure(&mut f, "recent_mfa");
    let (_, authorization) = f.authorize().await;
    let csrf = authorization["csrf_token"].as_str().unwrap();
    let body = serde_json::json!({"interaction":authorization["interaction"]}).to_string();
    let path = "/oauth/authorize/authentication";
    let (status, headers, state) = f.post(path, csrf, &body, "https://identity.example").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-store");
    assert!(!headers.contains_key("set-cookie"));
    assert_eq!(
        state,
        serde_json::json!({"minimum_authentication":"recent_mfa","needs_login":false,"needs_step_up":true,"proof_expires_at":null})
    );
    let (status, _, denied) = f
        .post(
            "/oauth/authorize/approve",
            csrf,
            &body,
            "https://identity.example",
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(denied["error"], "access_denied");
    let (_, code) = f.active_factor().await;
    assert_eq!(
        f.post(
            "/oauth/session/step-up/totp",
            authorization["session_csrf_token"].as_str().unwrap(),
            &serde_json::json!({"code":code}).to_string(),
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::OK
    );
    let state = f
        .post(path, csrf, &body, "https://identity.example")
        .await
        .2;
    assert_eq!(state["needs_step_up"], false);
    assert!(state["proof_expires_at"].is_string());
    let unchanged: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM identity_oauth_requests),(SELECT count(*) FROM identity_oauth_codes)").fetch_one(&f.db).await.unwrap();
    assert_eq!(unchanged, (1, 0));
    configure(&mut f, "recent_webauthn");
    let state = f
        .post(path, csrf, &body, "https://identity.example")
        .await
        .2;
    assert_eq!(state["minimum_authentication"], "recent_webauthn");
    assert_eq!(state["needs_step_up"], true);
    let (_, other) = session(&f.db).await;
    f.token = other;
    let state = f
        .post(path, csrf, &body, "https://identity.example")
        .await
        .2;
    assert_eq!(state["needs_login"], true);
    assert_eq!(state["needs_step_up"], false);
}

#[tokio::test]
async fn status_rejects_bad_origin_csrf_ambiguous_bodies_and_unavailable_storage() {
    let f = Fixture::new().await;
    let (_, authorization) = f.authorize().await;
    let csrf = authorization["csrf_token"].as_str().unwrap();
    let handle = authorization["interaction"].as_str().unwrap();
    let path = "/oauth/authorize/authentication";
    let body = serde_json::json!({"interaction":handle}).to_string();
    assert_eq!(
        f.post(path, csrf, "{", "https://account.example").await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.post(path, &random_secret(), &body, "https://identity.example")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    for body in [
        format!("[\"{handle}\"]"),
        format!("{{\"interaction\":\"{handle}\",\"interaction\":\"{handle}\"}}"),
        format!("{{\"interaction\":\"{handle}\",\"minimum_authentication\":\"primary\"}}"),
        "x".repeat(5000),
    ] {
        assert_eq!(
            f.post(path, csrf, &body, "https://identity.example")
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    f.db.close().await;
    let (status, _, error) = f.post(path, csrf, &body, "https://identity.example").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(error["error"], "temporarily_unavailable");
}
