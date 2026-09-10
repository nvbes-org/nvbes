use super::*;

fn direct_query() -> String {
    let input = test_fixtures::input();
    let mut url = reqwest::Url::parse("https://identity.example/").unwrap();
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
    url.query().unwrap().to_owned()
}

async fn get(f: &Fixture, query: &str, cookie: &str) -> (StatusCode, HeaderMap, serde_json::Value) {
    let request = Request::get(format!("/oauth/authorize?{query}"))
        .header("cookie", cookie)
        .body(Body::empty())
        .unwrap();
    let response = f.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 8192).await.unwrap();
    (
        status,
        headers,
        serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null),
    )
}

async fn push_raw(f: &Fixture, body: String) -> axum::response::Response {
    let tokens =
        Arc::new(crate::tokens::TokenService::new(crate::tokens::tests::config()).unwrap());
    let app = crate::oauth::http::token_router(
        f.db.clone(),
        Arc::new(clients()),
        tokens.clone(),
        crate::rate_limits::RateLimiter::new([53; 32]).unwrap(),
    );
    let request = Request::post("/oauth/par")
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:4000".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();
    app.oneshot(request).await.unwrap()
}

async fn push(f: &Fixture) -> String {
    let response = push_raw(f, direct_query()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    format!(
        "client_id=account-web&request_uri={}",
        body["request_uri"].as_str().unwrap()
    )
}

#[tokio::test]
async fn par_uses_the_same_strict_decoder_and_refuses_nested_requests() {
    let f = Fixture::new().await;
    for extra in [
        "client_id=account-web",
        "max_age=1&max_age=2",
        "request=unsupported",
        "request_uri=urn:ietf:params:oauth:request_uri:invalid",
    ] {
        let response = push_raw(&f, format!("{}&{extra}", direct_query())).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{extra}");
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(rows(&f).await, (0, 0, 0));
    }
    let response = push_raw(&f, "a".repeat(8193)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(rows(&f).await, (0, 0, 0));
    assert_eq!(
        push_raw(&f, format!("{}&max_age=300", direct_query()))
            .await
            .status(),
        StatusCode::CREATED
    );
    let age: i32 =
        sqlx::query_scalar("SELECT (parameters->>'max_age')::integer FROM identity_oauth_requests")
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(age, 300);
}

async fn rows(f: &Fixture) -> (i64, i64, i64) {
    sqlx::query_as("SELECT count(*) FILTER (WHERE kind='par'),count(*) FILTER (WHERE kind='authorization'),count(*) FILTER (WHERE browser_hash IS NOT NULL) FROM identity_oauth_requests")
        .fetch_one(&f.db).await.unwrap()
}

#[tokio::test]
async fn direct_numeric_max_age_is_preserved_and_ambiguous_queries_create_nothing() {
    let f = Fixture::new().await;
    let direct = direct_query();
    for extra in [
        "client_id=account-web",
        "%63lient_id=other",
        "scope=openid",
        "max_age=-1",
        "max_age=abc",
        "max_age=4294967296",
        "max_age=86401",
        "request=signed-but-unsupported",
        "max_age=1&max_age=2",
    ] {
        let response = get(&f, &format!("{direct}&{extra}"), &f.cookie()).await;
        assert_eq!(response.0, StatusCode::BAD_REQUEST, "{extra}");
        assert!(!response.1.contains_key("location"));
        assert_eq!(rows(&f).await, (0, 0, 0));
    }
    for age in [0, 300, 86400] {
        let response = get(&f, &format!("{direct}&max_age={age}"), &f.cookie()).await;
        assert_eq!(response.0, StatusCode::OK);
        let stored:i64 = sqlx::query_scalar("SELECT (parameters->>'max_age')::bigint FROM identity_oauth_requests WHERE handle_hash=$1")
            .bind(crate::oauth::store::hash(response.2["interaction"].as_str().unwrap())).fetch_one(&f.db).await.unwrap();
        assert_eq!(stored, age);
    }
}

#[tokio::test]
async fn par_creates_one_bound_interaction_and_rejects_replay_and_overrides() {
    let f = Fixture::new().await;
    let query = push(&f).await;
    assert_eq!(rows(&f).await, (1, 0, 0));
    for invalid in [
        format!("{query}&scope=openid"),
        format!("{query}&client_id=other"),
        query.replace("client_id=account-web", "client_id=unknown"),
    ] {
        assert!(get(&f, &invalid, &f.cookie()).await.0.is_client_error());
        assert_eq!(rows(&f).await, (1, 0, 0));
    }
    assert_eq!(get(&f, &query, &f.cookie()).await.0, StatusCode::OK);
    assert_eq!(rows(&f).await, (0, 1, 1));
    assert_eq!(
        get(&f, &query, &f.cookie()).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(rows(&f).await, (0, 1, 1));
}

#[tokio::test]
async fn browser_rejection_and_binding_failure_preserve_the_par_for_retry() {
    let f = Fixture::new().await;
    let query = push(&f).await;
    let cookie = format!("{}; __Host-nvbes-browser={}", f.cookie(), random_secret());
    assert_eq!(get(&f, &query, &cookie).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(rows(&f).await, (1, 0, 0));
    sqlx::query("CREATE FUNCTION fail_authorization_binding() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected binding failure'; END $$").execute(&f.db).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_authorization_binding BEFORE UPDATE ON identity_oauth_requests FOR EACH ROW EXECUTE FUNCTION fail_authorization_binding()").execute(&f.db).await.unwrap();
    for query in [&query, &direct_query()] {
        let response = get(&f, query, &f.cookie()).await;
        assert_eq!(response.0, StatusCode::SERVICE_UNAVAILABLE);
        assert!(!response.1.contains_key("set-cookie"));
        assert_eq!(rows(&f).await, (1, 0, 0));
    }
    sqlx::query("DROP TRIGGER fail_authorization_binding ON identity_oauth_requests")
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(get(&f, &query, &f.cookie()).await.0, StatusCode::OK);
    assert_eq!(rows(&f).await, (0, 1, 1));
}

#[tokio::test]
async fn concurrent_par_redemptions_bind_only_one_browser() {
    let f = Fixture::new().await;
    let query = push(&f).await;
    let cookie = f.cookie();
    let other = format!("__Host-nvbes-browser={}", random_secret());
    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(get(&f, &query, &cookie), get(&f, &query, &other))
    })
    .await
    .unwrap();
    assert_ne!(a.0 == StatusCode::OK, b.0 == StatusCode::OK);
    assert_eq!(
        if a.0 == StatusCode::OK { b.0 } else { a.0 },
        StatusCode::BAD_REQUEST
    );
    assert_eq!(rows(&f).await, (0, 1, 1));
}

#[tokio::test]
async fn silent_authorization_does_not_disguise_storage_failure_as_login_required() {
    let f = Fixture::new().await;
    sqlx::query("ALTER TABLE identity_oauth_consents RENAME TO unavailable_consents")
        .execute(&f.db)
        .await
        .unwrap();
    let response = get(&f, &format!("{}&prompt=none", direct_query()), &f.cookie()).await;
    assert_eq!(response.0, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!response.1.contains_key("location"));
    assert_eq!(response.2["error"], "temporarily_unavailable");
}
