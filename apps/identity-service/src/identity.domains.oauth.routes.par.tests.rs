use super::{consume_pushed_parameters, resolve_pushed_parameters};
use uuid::Uuid;

async fn test_redis_pool() -> nvbes_redis::RedisPool {
    crate::test_support::test_redis_pool().await
}

#[tokio::test]
async fn stores_and_resolves_pushed_authorization_request_via_redis() {
    let redis = test_redis_pool().await;
    let request_uri = format!(
        "urn:ietf:params:oauth:request_uri:gxpar_{}",
        Uuid::new_v4().simple()
    );
    let client_id = format!("client-{}", Uuid::new_v4());

    nvbes_redis::par::store_pushed_authorization_request(
        &redis,
        &nvbes_redis::par::CachedPushedAuthorizationRequest {
            request_uri: request_uri.clone(),
            client_id: client_id.clone(),
            parameters: serde_json::json!({
                "redirect_uri": "https://client.example.com/cb",
                "scope": "openid profile",
                "state": "state-123",
            })
            .as_object()
            .cloned()
            .expect("parameters should be an object"),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(60),
            used_at: None,
        },
    )
    .await
    .expect("par should store");

    let params = resolve_pushed_parameters(&redis, &request_uri, &client_id)
        .await
        .expect("par should resolve");
    assert_eq!(
        params.get("redirect_uri").and_then(|value| value.as_str()),
        Some("https://client.example.com/cb")
    );

    consume_pushed_parameters(&redis, &request_uri, &client_id)
        .await
        .expect("par should be consumed");

    let error = resolve_pushed_parameters(&redis, &request_uri, &client_id)
        .await
        .expect_err("used request_uri should be rejected");
    assert_eq!(error.code, "invalid_request_uri");
}
