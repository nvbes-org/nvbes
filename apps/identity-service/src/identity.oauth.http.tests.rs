use super::{authorization_router, router};
use crate::tokens::TokenService;
use crate::{browser::BrowserSecurity, mfa_crypto::MfaCrypto};
use axum::{
    Json, Router,
    body::Bytes,
    http::Method,
    routing::{any, post},
};

struct TestServer {
    url: String,
    task: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn start(app: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { url, task }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
async fn authorization_redirect_discards_post_body_for_success_and_denial() {
    let callback = TestServer::start(Router::new().route(
        "/callback",
        any(|method: Method, body: Bytes| async move {
            Json(serde_json::json!({"method": method.as_str(), "body_length": body.len()}))
        }),
    ))
    .await;
    for (key, value) in [("code", "issued-code"), ("error", "access_denied")] {
        let destination = format!("{}/callback?registered=value", callback.url);
        let source = TestServer::start(Router::new().route(
            "/consent",
            post(move || {
                let destination = destination.clone();
                async move {
                    super::redirect_with_result(&destination, key, value, "state&code=injected")
                }
            }),
        ))
        .await;
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let response = client
            .post(format!("{}/consent", source.url))
            .body("interaction=private-hosted-state")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let query: Vec<_> = response
            .url()
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert_eq!(
            query,
            vec![
                ("registered".to_owned(), "value".to_owned()),
                (key.to_owned(), value.to_owned()),
                ("state".to_owned(), "state&code=injected".to_owned()),
            ]
        );
        let received: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            received,
            serde_json::json!({"method": "GET", "body_length": 0})
        );
    }
}

#[test]
fn authorization_redirect_is_uncacheable_and_does_not_disclose_referrer() {
    for (key, value) in [
        ("code", "issued-code"),
        ("error", "login_required"),
        ("error", "consent_required"),
        ("error", "access_denied"),
    ] {
        let response =
            super::redirect_with_result("https://account.example/callback", key, value, "state")
                .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["pragma"], "no-cache");
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    }
    assert!(super::redirect_with_result("invalid URI", "code", "code", "state").is_err());
}

#[test]
fn http_module_is_kept_separate_from_mutating_authorization_routes() {
    let _router_type: fn(&str, std::sync::Arc<TokenService>) -> _ = router;
}

#[test]
fn authorization_router_requires_browser_and_mfa_runtime_state() {
    let _router_type: fn(
        sqlx::PgPool,
        std::sync::Arc<crate::oauth::clients::ClientRegistry>,
        BrowserSecurity,
        std::sync::Arc<MfaCrypto>,
    ) -> _ = authorization_router;
}
