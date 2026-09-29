use super::*;
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde_json::json;
use std::sync::Mutex;

#[tokio::test]
async fn account_decisions_are_authenticated_bound_uncached_and_fail_closed() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let principal = Uuid::new_v4();
    let account = BillingAccount {
        id: Uuid::new_v4(),
        account_type: AccountType::Team,
    };
    let expected = json!({"principal_id": principal, "account_id": account.id, "account_type":"team","allowed":true});
    let response = Arc::new(Mutex::new((StatusCode::OK, expected.to_string())));
    let app = Router::new().route("/internal/v1/billing/authorize", post({
        let response = response.clone();
        move |headers: HeaderMap, Json(body): Json<serde_json::Value>| {
            let response = response.lock().unwrap().clone();
            async move {
                assert_eq!(headers["authorization"], format!("Bearer {}", "a".repeat(64)));
                assert_eq!(body, json!({"principal_id":principal,"account_id":account.id,"account_type":"team"}));
                (response.0, [("content-type","application/json")], response.1)
            }
        }
    }));
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = AccountAuthority::new(&origin, &"a".repeat(64)).unwrap();
    client.require(principal, &account).await.unwrap();
    let mut denied = expected.clone();
    denied["allowed"] = json!(false);
    *response.lock().unwrap() = (StatusCode::OK, denied.to_string());
    assert!(matches!(
        client.require(principal, &account).await,
        Err(BillingError::AccountForbidden)
    ));
    for (field, value) in [
        ("principal_id", json!(Uuid::new_v4())),
        ("account_id", json!(Uuid::new_v4())),
        ("account_type", json!("principal")),
    ] {
        let mut changed = expected.clone();
        changed[field] = value;
        *response.lock().unwrap() = (StatusCode::OK, changed.to_string());
        assert!(matches!(
            client.require(principal, &account).await,
            Err(BillingError::AccountUnavailable)
        ));
    }
    for body in [
        "{}".into(),
        "{".into(),
        "x".repeat(1025),
        expected
            .to_string()
            .replace("\"allowed\":true", "\"allowed\":true,\"allowed\":false"),
    ] {
        *response.lock().unwrap() = (StatusCode::OK, body);
        assert!(matches!(
            client.require(principal, &account).await,
            Err(BillingError::AccountUnavailable)
        ));
    }
    for status in [
        StatusCode::FOUND,
        StatusCode::UNAUTHORIZED,
        StatusCode::TOO_MANY_REQUESTS,
        StatusCode::SERVICE_UNAVAILABLE,
    ] {
        *response.lock().unwrap() = (status, expected.to_string());
        assert!(matches!(
            client.require(principal, &account).await,
            Err(BillingError::AccountUnavailable)
        ));
    }
    *response.lock().unwrap() = (StatusCode::OK, expected.to_string());
    let permits = client.permits.acquire_many(16).await.unwrap();
    assert!(matches!(
        client.require(principal, &account).await,
        Err(BillingError::AccountUnavailable)
    ));
    drop(permits);
    client.require(principal, &account).await.unwrap();
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
    assert!(matches!(
        client.require(principal, &account).await,
        Err(BillingError::AccountUnavailable)
    ));
}

#[test]
fn authority_configuration_rejects_unsafe_origins_and_weak_credentials() {
    for origin in [
        "http://account.example",
        "https://user:pass@account.example",
        "https://account.example/?q=x",
        "https://account.example/#x",
        "https://account.example/path",
    ] {
        assert!(AccountAuthority::new(origin, &"a".repeat(64)).is_err());
    }
    for secret in ["".into(), "a".repeat(63), "g".repeat(64), "A".repeat(64)] {
        assert!(AccountAuthority::new("https://account.example", &secret).is_err());
    }
}
