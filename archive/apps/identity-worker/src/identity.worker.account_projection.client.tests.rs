use std::sync::Mutex;

use reqwest::StatusCode;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{AccountProjectionClient, DispatchError};

static ENV_LOCK: Mutex<()> = Mutex::new(());
const TOKEN: &str = "identity-worker-account-token-0001";

#[test]
fn status_errors_have_stable_retry_and_classification_contracts() {
    let cases = [
        (
            StatusCode::TOO_MANY_REQUESTS,
            true,
            "account_projection_rejected",
        ),
        (
            StatusCode::BAD_GATEWAY,
            true,
            "account_projection_unavailable",
        ),
        (
            StatusCode::UNAUTHORIZED,
            false,
            "account_projection_unauthorized",
        ),
        (StatusCode::CONFLICT, false, "account_projection_conflict"),
        (
            StatusCode::BAD_REQUEST,
            false,
            "account_projection_rejected",
        ),
        (
            StatusCode::PERMANENT_REDIRECT,
            false,
            "account_projection_unavailable",
        ),
    ];

    for (status, retryable, code) in cases {
        let error = DispatchError::from_status(status);
        assert_eq!(error.is_retryable(), retryable, "status {status}");
        assert_eq!(error.code(), code, "status {status}");
        assert_eq!(error.to_string(), code);
    }

    let transient = DispatchError::transient("transport_failed");
    assert!(transient.is_retryable());
    assert_eq!(transient.code(), "transport_failed");
}

#[test]
fn local_environments_receive_safe_defaults() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    clear_environment();

    for environment in ["development", "test"] {
        let client = AccountProjectionClient::from_env(environment).expect("local defaults");
        assert_eq!(
            client.endpoint.as_str(),
            "http://localhost:4001/internal/v1/identity-registrations"
        );
        assert_eq!(client.token, "development-account-provisioning-token");
    }

    clear_environment();
}

#[test]
fn production_requires_endpoint_and_token() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    clear_environment();

    let missing_endpoint = AccountProjectionClient::from_env("production")
        .err()
        .expect("endpoint must be required");
    assert!(
        missing_endpoint
            .to_string()
            .contains("BASE_URL is required")
    );

    set_env(
        "NVBES_ACCOUNT_SERVICE_BASE_URL",
        "https://account.example.com",
    );
    let missing_token = AccountProjectionClient::from_env("production")
        .err()
        .expect("token must be required");
    assert!(
        missing_token
            .to_string()
            .contains("PROVISIONING_TOKEN is required")
    );

    clear_environment();
}

#[test]
fn endpoint_and_token_validation_rejects_ambiguous_configuration() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    let invalid_urls = [
        "ftp://account.example.com",
        "https://user@account.example.com",
        "https://account.example.com?tenant=other",
        "https://account.example.com#fragment",
        "/relative",
    ];

    for url in invalid_urls {
        clear_environment();
        set_env("NVBES_ACCOUNT_SERVICE_BASE_URL", url);
        set_env("NVBES_ACCOUNT_PROVISIONING_TOKEN", TOKEN);
        assert!(
            AccountProjectionClient::from_env("production").is_err(),
            "URL {url}"
        );
    }

    clear_environment();
    set_env(
        "NVBES_ACCOUNT_SERVICE_BASE_URL",
        "https://account.example.com/base",
    );
    set_env("NVBES_ACCOUNT_PROVISIONING_TOKEN", " too-short ");
    let error = AccountProjectionClient::from_env("production")
        .err()
        .expect("short token must fail");
    assert!(error.to_string().contains("at least 32"));
    clear_environment();
}

#[test]
fn valid_configuration_normalizes_the_endpoint_and_token() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    clear_environment();
    set_env(
        "NVBES_ACCOUNT_SERVICE_BASE_URL",
        " https://account.example.com/base ",
    );
    set_env("NVBES_ACCOUNT_PROVISIONING_TOKEN", &format!("  {TOKEN}  "));

    let client = AccountProjectionClient::from_env("production").expect("valid client");

    assert_eq!(
        client.endpoint.as_str(),
        "https://account.example.com/internal/v1/identity-registrations"
    );
    assert_eq!(client.token, TOKEN);
    clear_environment();
}

#[tokio::test]
async fn dispatch_sends_authenticated_json_and_accepts_success() {
    let (endpoint, request) = spawn_http_response("204 No Content").await;
    let client = client(endpoint);

    client
        .dispatch(&json!({"principal_id": "principal-1"}))
        .await
        .expect("dispatch");

    let request = request.await.expect("request task");
    assert!(request.starts_with("POST /internal/v1/identity-registrations HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {TOKEN}"))
    );
    assert!(request.contains("\"principal_id\":\"principal-1\""));
}

#[tokio::test]
async fn dispatch_classifies_remote_and_transport_failures() {
    for (status, retryable, code) in [
        ("429 Too Many Requests", true, "account_projection_rejected"),
        (
            "503 Service Unavailable",
            true,
            "account_projection_unavailable",
        ),
        ("401 Unauthorized", false, "account_projection_unauthorized"),
        ("409 Conflict", false, "account_projection_conflict"),
    ] {
        let (endpoint, request) = spawn_http_response(status).await;
        let error = client(endpoint)
            .dispatch(&json!({"event": status}))
            .await
            .expect_err("remote failure");
        request.await.expect("request task");
        assert_eq!(error.is_retryable(), retryable);
        assert_eq!(error.code(), code);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let endpoint = reqwest::Url::parse(&format!("http://{}", listener.local_addr().expect("addr")))
        .expect("URL");
    drop(listener);
    let error = client(endpoint)
        .dispatch(&json!({"event": "transport"}))
        .await
        .expect_err("transport failure");
    assert!(error.is_retryable());
    assert_eq!(error.code(), "account_projection_unreachable");
}

fn client(mut endpoint: reqwest::Url) -> AccountProjectionClient {
    endpoint.set_path("/internal/v1/identity-registrations");
    AccountProjectionClient {
        http: reqwest::Client::new(),
        endpoint,
        token: TOKEN.to_string(),
    }
}

async fn spawn_http_response(
    status: &'static str,
) -> (reqwest::Url, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let endpoint = reqwest::Url::parse(&format!("http://{}", listener.local_addr().expect("addr")))
        .expect("URL");
    let request = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut bytes = vec![0; 8192];
        let read = stream.read(&mut bytes).await.expect("read request");
        let request = String::from_utf8(bytes[..read].to_vec()).expect("UTF-8 request");
        let response =
            format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write response");
        request
    });
    (endpoint, request)
}

fn clear_environment() {
    remove_env("NVBES_ACCOUNT_SERVICE_BASE_URL");
    remove_env("NVBES_ACCOUNT_PROVISIONING_TOKEN");
}

fn set_env(name: &str, value: &str) {
    unsafe { std::env::set_var(name, value) };
}

fn remove_env(name: &str) {
    unsafe { std::env::remove_var(name) };
}
