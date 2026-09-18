use super::*;

#[tokio::test]
async fn unavailable_claim_store_rolls_back_proof_and_allows_same_proof_retry() {
    let f = Fixture::new("openid email", true, tokens_policy::USERINFO_AUDIENCE).await;
    let proof = f.proof("GET", Some(&f.token), &f.endpoint());
    sqlx::query("ALTER TABLE identity_login_identifiers RENAME TO unavailable_identifiers")
        .execute(&f.db)
        .await
        .unwrap();
    let failed = f.request("GET", &f.token, "DPoP", Some(&proof)).await;
    assert_eq!(failed.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        failed.2,
        serde_json::json!({"error":"temporarily_unavailable"})
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_dpop_replay_keys")
        .fetch_one(&f.db)
        .await
        .unwrap();
    assert_eq!(count, 1);
    sqlx::query("ALTER TABLE unavailable_identifiers RENAME TO identity_login_identifiers")
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.request("GET", &f.token, "DPoP", Some(&proof)).await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn revoked_session_and_narrowed_client_policy_invalidate_userinfo_immediately() {
    let mut f = Fixture::new("openid email", false, tokens_policy::USERINFO_AUDIENCE).await;
    let original = f.clients.clone();
    let mut client = serde_json::to_value(f.clients.get("account-web").unwrap()).unwrap();
    client["resources"][f.endpoint()]["scopes"] = serde_json::json!(["openid"]);
    f.clients = Arc::new(
        ClientRegistry::from_json(&serde_json::json!([client]).to_string(), true).unwrap(),
    );
    assert_eq!(
        f.request("GET", &f.token, "Bearer", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    f.clients = original;
    assert_eq!(
        f.request("GET", &f.token, "Bearer", None).await.0,
        StatusCode::OK
    );
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE principal_id=$1")
        .bind(f.principal)
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.request("GET", &f.token, "Bearer", None).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn duplicate_headers_non_ascii_credentials_and_alternate_transports_are_rejected() {
    let f = Fixture::new("openid", false, tokens_policy::USERINFO_AUDIENCE).await;
    for case in 0..7 {
        let mut request = Request::builder()
            .method("POST")
            .uri(if case == 0 {
                "/oauth/userinfo?access_token=secret"
            } else {
                "/oauth/userinfo"
            })
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:6000".parse::<std::net::SocketAddr>().unwrap(),
            ))
            .header("authorization", format!("Bearer {}", f.token));
        match case {
            2 => {
                request = request.header("authorization", "Bearer second");
            }
            3 => {
                request = request.header("dpop", "one").header("dpop", "two");
            }
            4 => {
                request.headers_mut().unwrap().insert(
                    "authorization",
                    axum::http::HeaderValue::from_bytes(b"Bearer \xff").unwrap(),
                );
            }
            5 => {
                request
                    .headers_mut()
                    .unwrap()
                    .insert("authorization", "Bearer ".parse().unwrap());
            }
            6 => {
                request
                    .headers_mut()
                    .unwrap()
                    .insert("authorization", "Bearer two tokens".parse().unwrap());
            }
            _ => {}
        }
        let body = if case == 1 {
            Body::from("access_token=secret")
        } else {
            Body::empty()
        };
        let response = token_router(
            f.db.clone(),
            f.clients.clone(),
            f.service.clone(),
            f.limiter.clone(),
        )
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "case {case}");
        assert_eq!(response.headers()["cache-control"], "no-store");
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(json, serde_json::json!({"error":"invalid_request"}));
    }
}
