use super::*;
use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{any, post},
    Form, Router,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

#[derive(Clone)]
struct Reply {
    status: StatusCode,
    body: String,
    content_type: &'static str,
    delay: Duration,
}
struct Fixture {
    client: IntrospectionClient,
    expected: ExpectedToken,
    reply: Arc<Mutex<Reply>>,
    calls: Arc<AtomicUsize>,
    redirects: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expected = ExpectedToken {
            sub: "principal".into(),
            sid: "session".into(),
            grant_id: "grant".into(),
            jti: "token-id".into(),
            iss: issuer.clone(),
            aud: "nvbes-billing-service".into(),
            client_id: "account-web".into(),
            scope: "billing:read".into(),
            exp: now + 600,
            iat: now,
            nbf: now,
            token_type: "Bearer".into(),
            cnf: None,
        };
        let mut value = serde_json::to_value(&expected).unwrap();
        value["active"] = json!(true);
        let reply = Arc::new(Mutex::new(Reply {
            status: StatusCode::OK,
            body: value.to_string(),
            content_type: "application/json",
            delay: Duration::ZERO,
        }));
        let calls = Arc::new(AtomicUsize::new(0));
        let redirects = Arc::new(AtomicUsize::new(0));
        let app = Router::new()
            .route(
                "/oauth/introspect",
                post({
                    let reply = reply.clone();
                    let calls = calls.clone();
                    let target = format!("{issuer}/redirect-target");
                    move |headers: HeaderMap, Form(body): Form<BTreeMap<String, String>>| {
                        let reply = reply.lock().unwrap().clone();
                        let calls = calls.clone();
                        let target = target.clone();
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            assert_eq!(
                                headers["authorization"],
                                format!(
                                    "Basic {}",
                                    STANDARD.encode(format!("billing-api:{}", "A".repeat(43)))
                                )
                            );
                            assert_eq!(body.get("token").unwrap(), "token-under-test");
                            assert_eq!(body.get("token_type_hint").unwrap(), "access_token");
                            tokio::time::sleep(reply.delay).await;
                            (
                                reply.status,
                                [
                                    ("content-type", reply.content_type.to_owned()),
                                    ("location", target),
                                ],
                                reply.body,
                            )
                                .into_response()
                        }
                    }
                }),
            )
            .route(
                "/redirect-target",
                any({
                    let redirects = redirects.clone();
                    move || {
                        redirects.fetch_add(1, Ordering::SeqCst);
                        async { StatusCode::OK }
                    }
                }),
            );
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            client: IntrospectionClient::new(&issuer, "billing-api", &"A".repeat(43)).unwrap(),
            expected,
            reply,
            calls,
            redirects,
            server,
        }
    }
    async fn check(&self) -> Result<bool, IntrospectionError> {
        self.client
            .is_active("token-under-test", &self.expected)
            .await
    }
}

#[tokio::test]
async fn every_request_observes_current_activity_without_positive_cache() {
    let f = Fixture::new().await;
    assert!(f.check().await.unwrap());
    f.reply.lock().unwrap().body = json!({"active":false}).to_string();
    assert!(!f.check().await.unwrap());
    assert_eq!(f.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn malformed_mismatched_and_oversized_responses_never_authorize() {
    let f = Fixture::new().await;
    for body in [
        "{}".into(),
        "not-json".into(),
        json!({"active":"true"}).to_string(),
        json!({"active":true}).to_string(),
        "{\"active\":false,\"active\":true}".into(),
        "x".repeat(16_385),
    ] {
        f.reply.lock().unwrap().body = body;
        assert!(f.check().await.is_err());
    }
    for field in [
        "sub",
        "sid",
        "grant_id",
        "jti",
        "iss",
        "aud",
        "scope",
        "client_id",
        "token_type",
    ] {
        let mut value = serde_json::to_value(&f.expected).unwrap();
        value["active"] = json!(true);
        value[field] = json!("another-value");
        f.reply.lock().unwrap().body = value.to_string();
        assert!(f.check().await.is_err(), "accepted {field}");
    }
    let mut value = serde_json::to_value(&f.expected).unwrap();
    value["active"] = json!(true);
    value["exp"] = json!(1);
    f.reply.lock().unwrap().body = value.to_string();
    assert!(f.check().await.is_err());
    f.reply.lock().unwrap().body = json!({"active":false}).to_string();
    f.reply.lock().unwrap().content_type = "text/html";
    assert!(f.check().await.is_err());
}

#[tokio::test]
async fn errors_redirects_timeouts_and_capacity_exhaustion_are_unavailable() {
    let f = Fixture::new().await;
    for status in [
        StatusCode::UNAUTHORIZED,
        StatusCode::TOO_MANY_REQUESTS,
        StatusCode::SERVICE_UNAVAILABLE,
        StatusCode::TEMPORARY_REDIRECT,
    ] {
        f.reply.lock().unwrap().status = status;
        assert!(f.check().await.is_err());
    }
    assert_eq!(f.redirects.load(Ordering::SeqCst), 0);
    {
        let mut reply = f.reply.lock().unwrap();
        reply.status = StatusCode::OK;
        reply.delay = Duration::from_secs(4);
    }
    assert!(tokio::time::timeout(Duration::from_secs(3), f.check())
        .await
        .unwrap()
        .is_err());
    let permits = f.client.capacity.acquire_many(16).await.unwrap();
    assert!(f.check().await.is_err());
    drop(permits);
}

#[test]
fn configuration_refuses_remote_http_userinfo_queries_and_weak_credentials() {
    for issuer in [
        "http://identity.example",
        "https://user@identity.example",
        "https://identity.example?query=1",
        "https://identity.example/#fragment",
    ] {
        assert!(IntrospectionClient::new(issuer, "billing-api", &"A".repeat(43)).is_err());
    }
    assert!(
        IntrospectionClient::new("https://identity.example", "bad:id", &"A".repeat(43)).is_err()
    );
    assert!(IntrospectionClient::new("https://identity.example", "billing-api", "short").is_err());
}
