use base64::Engine as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{KmsSignResponse, sign_digest_at, verify_signature_at};
use crate::worker::audit_anchor::AuditAnchorConfig;

#[tokio::test]
async fn signing_sends_the_digest_and_accepts_a_matching_non_empty_signature() {
    let config = config();
    let (endpoint, request) = json_server(
        "200 OK",
        r#"{"key_id":"key-1","signature":"signed-digest"}"#,
    )
    .await;

    let signature = sign_digest_at(&config, b"digest", &endpoint)
        .await
        .expect("signature");

    assert_eq!(signature.key_id, "key-1");
    assert_eq!(signature.signature, "signed-digest");
    let request = request.await.expect("request");
    assert!(
        request
            .to_ascii_lowercase()
            .contains("x-auth-token: auth-token")
    );
    assert!(request.contains(&base64::engine::general_purpose::STANDARD.encode(b"digest")));
}

#[tokio::test]
async fn signing_rejects_http_errors_and_invalid_responses() {
    for (status, body, expected) in [
        ("503 Service Unavailable", "{}", "status 503"),
        (
            "200 OK",
            r#"{"key_id":"other","signature":"sig"}"#,
            "invalid signing response",
        ),
        (
            "200 OK",
            r#"{"key_id":"key-1","signature":"  "}"#,
            "invalid signing response",
        ),
        ("200 OK", "not-json", "decoding"),
    ] {
        let (endpoint, request) = json_server(status, body).await;
        let error = sign_digest_at(&config(), b"digest", &endpoint)
            .await
            .expect_err("invalid response");
        request.await.expect("request");
        assert!(
            error
                .to_string()
                .to_ascii_lowercase()
                .contains(&expected.to_ascii_lowercase()),
            "{error}"
        );
    }
}

#[tokio::test]
async fn verification_accepts_only_a_valid_response_for_the_configured_key() {
    let signature = KmsSignResponse {
        key_id: "key-1".to_string(),
        signature: "signed-digest".to_string(),
    };
    let (endpoint, request) = json_server("200 OK", r#"{"key_id":"key-1","valid":true}"#).await;

    verify_signature_at(&config(), b"digest", &signature, &endpoint)
        .await
        .expect("verification");

    let request = request.await.expect("request");
    assert!(request.contains("signed-digest"));
}

#[tokio::test]
async fn verification_fails_closed_for_http_key_and_validity_errors() {
    let signature = KmsSignResponse {
        key_id: "key-1".to_string(),
        signature: "signed-digest".to_string(),
    };
    for (status, body, expected) in [
        ("500 Internal Server Error", "{}", "status 500"),
        (
            "200 OK",
            r#"{"key_id":"other","valid":true}"#,
            "failed closed",
        ),
        (
            "200 OK",
            r#"{"key_id":"key-1","valid":false}"#,
            "failed closed",
        ),
    ] {
        let (endpoint, request) = json_server(status, body).await;
        let error = verify_signature_at(&config(), b"digest", &signature, &endpoint)
            .await
            .expect_err("verification must fail");
        request.await.expect("request");
        assert!(error.to_string().contains(expected), "{error}");
    }
}

fn config() -> AuditAnchorConfig {
    AuditAnchorConfig {
        region: "fr-par".to_string(),
        kms_key_id: "key-1".to_string(),
        kms_auth_token: "auth-token".to_string(),
        bucket: "bucket".to_string(),
        endpoint: "http://storage".to_string(),
        access_key: "access".to_string(),
        secret_key: "secret".to_string(),
    }
}

async fn json_server(
    status: &'static str,
    body: &'static str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let endpoint = format!("http://{}", listener.local_addr().expect("addr"));
    let request = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut bytes = vec![0; 8192];
        let read = stream.read(&mut bytes).await.expect("read");
        let request = String::from_utf8(bytes[..read].to_vec()).expect("request UTF-8");
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("respond");
        request
    });
    (endpoint, request)
}
