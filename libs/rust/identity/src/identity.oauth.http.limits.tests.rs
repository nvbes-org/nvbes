use super::*;
use crate::{
    oauth::http::token_router,
    rate_limits::{Category, RateLimiter},
    tokens::TokenService,
};

fn token_app(f: &Fixture) -> Router {
    let tokens = Arc::new(TokenService::new(crate::tokens::tests::config()).unwrap());
    token_router(
        f.db.clone(),
        Arc::new(clients()),
        tokens.clone(),
        RateLimiter::new([53; 32]).unwrap(),
    )
}

#[tokio::test]
async fn tcp_transport_supplies_the_peer_used_for_throttling() {
    let f = Fixture::new().await;
    f.authorize().await;
    sqlx::query("UPDATE identity_rate_buckets SET attempts=120 WHERE category='protocol_source'")
        .execute(&f.db)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let app = token_app(&f);
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = stopped.await;
        })
        .await
        .unwrap();
    });
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
        .get(format!("http://{address}/oauth/userinfo"))
        .header("x-forwarded-for", "203.0.113.99")
        .send()
        .await;
    stop.send(()).unwrap();
    server.await.unwrap();
    let response = response.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()["retry-after"], "60");
}

#[tokio::test]
async fn every_protocol_route_rejects_exhausted_sources_before_parsing() {
    let f = Fixture::new().await;
    f.authorize().await;
    sqlx::query("UPDATE identity_rate_buckets SET attempts=120 WHERE category='protocol_source'")
        .execute(&f.db)
        .await
        .unwrap();
    let app = f.app.clone().merge(token_app(&f));
    for (method, path) in [
        ("GET", "/oauth/authorize"),
        ("POST", "/oauth/authorize/login"),
        ("POST", "/oauth/authorize/approve"),
        ("POST", "/oauth/authorize/authentication"),
        ("POST", "/oauth/authorize/deny"),
        ("POST", "/oauth/logout"),
        ("POST", "/oauth/session/step-up/totp"),
        ("POST", "/oauth/token"),
        ("POST", "/oauth/par"),
        ("GET", "/oauth/userinfo"),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:4567".parse::<std::net::SocketAddr>().unwrap(),
            ))
            .header("x-forwarded-for", "198.51.100.1")
            .header("forwarded", "for=198.51.100.2")
            .header("cf-connecting-ip", "198.51.100.3")
            .body(Body::from("malformed"))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS, "{path}");
        assert_eq!(response.headers()["retry-after"], "60");
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert!(!response.headers().contains_key("set-cookie"));
    }
}

#[tokio::test]
async fn missing_transport_or_unavailable_quota_store_fails_closed() {
    let f = Fixture::new().await;
    let app = token_app(&f);
    let request = Request::post("/oauth/token")
        .header("x-forwarded-for", "127.0.0.1")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::query("DROP TABLE identity_rate_buckets")
        .execute(&f.db)
        .await
        .unwrap();
    let request = Request::post("/oauth/token")
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:4567".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.oneshot(request).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn normalized_login_quota_survives_failed_passwords_and_allows_retry_after_expiry() {
    let f = Fixture::new().await;
    let email = "quota@example.invalid";
    let password = "Quota-test-password-long!";
    let principal = crate::auth::create_synthetic_identity(&f.db, email, password)
        .await
        .unwrap();
    let (_, authorization) = f.authorize().await;
    let csrf = authorization["csrf_token"].as_str().unwrap();
    for attempt in 0..6 {
        let email = if attempt % 2 == 0 {
            email
        } else {
            " QUOTA@EXAMPLE.INVALID "
        };
        let body = serde_json::json!({"interaction":authorization["interaction"],"email":email,"password":if attempt==5 {password} else {"incorrect"}}).to_string();
        let (status, headers, _) = f
            .post(
                "/oauth/authorize/login",
                csrf,
                &body,
                "https://identity.example",
            )
            .await;
        assert_eq!(
            status,
            if attempt == 5 {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::BAD_REQUEST
            }
        );
        if attempt == 5 {
            assert_eq!(headers["retry-after"], "600");
        }
    }
    let sessions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity_sessions WHERE principal_id=$1")
            .bind(principal)
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(sessions, 0);
    sqlx::query("UPDATE identity_rate_buckets SET reset_at=clock_timestamp()-interval '1 second' WHERE category='login_account'").execute(&f.db).await.unwrap();
    let body = serde_json::json!({"interaction":authorization["interaction"],"email":email,"password":password}).to_string();
    assert_eq!(
        f.post(
            "/oauth/authorize/login",
            csrf,
            &body,
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn totp_quota_is_shared_by_sessions_of_the_same_principal() {
    let mut f = Fixture::new().await;
    let (_, authorization) = f.authorize().await;
    let (_, code) = f.active_factor().await;
    let csrf = authorization["session_csrf_token"].as_str().unwrap();
    let body = serde_json::json!({"code":code}).to_string();
    for attempt in 0..5 {
        assert_eq!(
            f.post(
                "/oauth/session/step-up/totp",
                csrf,
                &body,
                "https://identity.example"
            )
            .await
            .0,
            if attempt == 0 {
                StatusCode::OK
            } else {
                StatusCode::BAD_REQUEST
            }
        );
    }
    let (other, token) = session(&f.db).await;
    sqlx::query("UPDATE identity_sessions SET principal_id=(SELECT principal_id FROM identity_sessions WHERE id=$1) WHERE id=$2").bind(f.session).bind(other).execute(&f.db).await.unwrap();
    f.token = token;
    let (_, authorization) = f.authorize().await;
    let csrf = authorization["session_csrf_token"].as_str().unwrap();
    let (status, headers, _) = f
        .post(
            "/oauth/session/step-up/totp",
            csrf,
            &body,
            "https://identity.example",
        )
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(headers["retry-after"], "600");
    let untouched: bool =
        sqlx::query_scalar("SELECT step_up_expires_at IS NULL FROM identity_sessions WHERE id=$1")
            .bind(other)
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert!(untouched);
}

#[tokio::test]
async fn password_source_limit_applies_before_browser_and_body_validation() {
    let f = Fixture::new().await;
    let limiter = RateLimiter::new([53; 32]).unwrap();
    for _ in 0..30 {
        limiter
            .check(&f.db, Category::LoginSource, "127.0.0.1")
            .await
            .unwrap();
    }
    assert_eq!(
        f.post(
            "/oauth/authorize/login",
            &random_secret(),
            "{",
            "https://identity.example"
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
}
