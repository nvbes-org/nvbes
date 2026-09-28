use openssl::{hash::MessageDigest, sign::Signer};

use super::{SnsMessage, WebhookVerifier, canonical_message};
use crate::config::WebhookTrustConfig;
use crate::webhook_verify_fixtures::{certificate_authority, leaf_certificate};

#[tokio::test]
async fn verification_accepts_a_valid_signature_from_a_seeded_certificate() {
    let (authority, authority_key) = certificate_authority("nvbes-test-ca");
    let (leaf, leaf_key) = leaf_certificate("sns.scaleway.test", &authority, &authority_key);
    let topic = "arn:scw:sns:fr-par:test:topic";
    let cert_url = "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem";
    let mut message = SnsMessage {
        message_type: "Notification".into(),
        message_id: "message-id".into(),
        topic_arn: topic.into(),
        message: r#"{"id":"provider-event-1","type":"delivery"}"#.into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        signature_version: "1".into(),
        signature: String::new(),
        signing_cert_url: cert_url.into(),
        subject: Some("delivery".into()),
        token: None,
        subscribe_url: None,
    };
    let canonical = canonical_message(&message).unwrap();
    let mut signer = Signer::new(MessageDigest::sha1(), &leaf_key).unwrap();
    signer.update(canonical.as_bytes()).unwrap();
    message.signature = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        signer.sign_to_vec().unwrap(),
    );

    let verifier = WebhookVerifier::new(&WebhookTrustConfig {
        topic_arn: topic.to_string(),
        ca_bundle_pem: authority.to_pem().unwrap(),
        signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
        confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
    })
    .unwrap();
    verifier.seed_certificate(cert_url, leaf).await;

    let body = serde_json::to_vec(&serde_json::json!({
        "Type": message.message_type,
        "MessageId": message.message_id,
        "TopicArn": message.topic_arn,
        "Message": message.message,
        "Timestamp": message.timestamp,
        "SignatureVersion": message.signature_version,
        "Signature": message.signature,
        "SigningCertURL": message.signing_cert_url,
        "Subject": message.subject,
    }))
    .unwrap();
    let verified = verifier.verify(&body).await.expect("valid signature");
    assert_eq!(verified.message_id, "message-id");
}

#[tokio::test]
async fn verification_rejects_an_invalid_signature_with_a_seeded_certificate() {
    let (authority, authority_key) = certificate_authority("nvbes-test-ca");
    let (leaf, _) = leaf_certificate("sns.scaleway.test", &authority, &authority_key);
    let topic = "arn:scw:sns:fr-par:test:topic";
    let cert_url = "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem";
    let verifier = WebhookVerifier::new(&WebhookTrustConfig {
        topic_arn: topic.to_string(),
        ca_bundle_pem: authority.to_pem().unwrap(),
        signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
        confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
    })
    .unwrap();
    verifier.seed_certificate(cert_url, leaf).await;

    let body = serde_json::to_vec(&serde_json::json!({
        "Type": "Notification",
        "MessageId": "message-id",
        "TopicArn": topic,
        "Message": "{}",
        "Timestamp": chrono::Utc::now().to_rfc3339(),
        "SignatureVersion": "1",
        "Signature": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"not-a-sig"),
        "SigningCertURL": cert_url,
    }))
    .unwrap();
    let error = verifier.verify(&body).await.expect_err("bad signature");
    assert!(
        matches!(
            error,
            super::SnsVerificationError::InvalidSignature
                | super::SnsVerificationError::Cryptography(_)
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn verification_rejects_invalid_signature_encoding_after_certificate_lookup() {
    let (authority, authority_key) = certificate_authority("nvbes-test-ca");
    let (leaf, _) = leaf_certificate("sns.scaleway.test", &authority, &authority_key);
    let topic = "arn:scw:sns:fr-par:test:topic";
    let cert_url = "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem";
    let verifier = WebhookVerifier::new(&WebhookTrustConfig {
        topic_arn: topic.to_string(),
        ca_bundle_pem: authority.to_pem().unwrap(),
        signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
        confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
    })
    .unwrap();
    verifier.seed_certificate(cert_url, leaf).await;

    let body = serde_json::to_vec(&serde_json::json!({
        "Type": "Notification",
        "MessageId": "message-id",
        "TopicArn": topic,
        "Message": "{}",
        "Timestamp": chrono::Utc::now().to_rfc3339(),
        "SignatureVersion": "1",
        "Signature": "!!!!",
        "SigningCertURL": cert_url,
    }))
    .unwrap();
    let error = verifier.verify(&body).await.expect_err("bad encoding");
    assert_eq!(error.outcome(), "invalid_signature_encoding");
}

#[test]
fn confirmation_url_rejects_disallowed_hosts_and_parameters() {
    let (authority, _) = certificate_authority("nvbes-test-ca");
    let verifier = WebhookVerifier::new(&WebhookTrustConfig {
        topic_arn: "arn:scw:sns:fr-par:test:topic".to_string(),
        ca_bundle_pem: authority.to_pem().unwrap(),
        signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
        confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
    })
    .unwrap();
    let topic = "arn:scw:sns:fr-par:test:topic";
    let token = "confirmation-token";
    let mut message = SnsMessage {
        message_type: "SubscriptionConfirmation".into(),
        message_id: "id".into(),
        topic_arn: topic.into(),
        message: "confirm".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        signature_version: "1".into(),
        signature: "signature".into(),
        signing_cert_url: "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem".into(),
        subject: None,
        token: Some(token.into()),
        subscribe_url: Some(format!(
            "https://evil.example/?Action=ConfirmSubscription&TopicArn={topic}&Token={token}"
        )),
    };
    assert!(verifier.confirmation_url(&message).is_err());

    message.subscribe_url = Some(format!(
        "https://sns.mnq.fr-par.scaleway.com/?Action=Subscribe&TopicArn={topic}&Token={token}"
    ));
    assert!(verifier.confirmation_url(&message).is_err());

    message.subscribe_url = Some(format!(
        "https://sns.mnq.fr-par.scaleway.com/?Action=ConfirmSubscription&TopicArn={topic}&Token=other"
    ));
    assert!(verifier.confirmation_url(&message).is_err());

    message.subscribe_url = Some(format!(
        "https://sns.mnq.fr-par.scaleway.com:8443/?Action=ConfirmSubscription&TopicArn={topic}&Token={token}"
    ));
    assert!(verifier.confirmation_url(&message).is_err());

    message.subscribe_url = Some(format!(
        "https://user:pass@sns.mnq.fr-par.scaleway.com/?Action=ConfirmSubscription&TopicArn={topic}&Token={token}"
    ));
    assert!(verifier.confirmation_url(&message).is_err());

    message.subscribe_url = Some(format!(
        "https://sns.mnq.fr-par.scaleway.com/?Action=ConfirmSubscription&TopicArn={topic}&Token={token}#frag"
    ));
    assert!(verifier.confirmation_url(&message).is_err());

    message.subscribe_url = Some(format!(
        "https://sns.mnq.fr-par.scaleway.com/?Action=ConfirmSubscription&TopicArn=other&Token={token}"
    ));
    assert!(verifier.confirmation_url(&message).is_err());

    message.token = None;
    message.subscribe_url = Some(format!(
        "https://sns.mnq.fr-par.scaleway.com/?Action=ConfirmSubscription&TopicArn={topic}&Token={token}"
    ));
    assert!(verifier.confirmation_url(&message).is_err());
    assert!(canonical_message(&message).is_err());

    message.timestamp = (chrono::Utc::now() + chrono::Duration::minutes(6)).to_rfc3339();
    message.token = Some(token.into());
    assert!(verifier.validate_envelope(&message).is_err());
}

#[tokio::test]
async fn seeded_certificate_is_reused_without_a_second_network_fetch() {
    let (authority, authority_key) = certificate_authority("nvbes-test-ca");
    let (leaf, leaf_key) = leaf_certificate("sns.scaleway.test", &authority, &authority_key);
    let topic = "arn:scw:sns:fr-par:test:topic";
    let cert_url = "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem";
    let verifier = WebhookVerifier::new(&WebhookTrustConfig {
        topic_arn: topic.to_string(),
        ca_bundle_pem: authority.to_pem().unwrap(),
        signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
        confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
    })
    .unwrap();
    verifier.seed_certificate(cert_url, leaf).await;

    let mut message = SnsMessage {
        message_type: "Notification".into(),
        message_id: "cached".into(),
        topic_arn: topic.into(),
        message: "{}".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        signature_version: "1".into(),
        signature: String::new(),
        signing_cert_url: cert_url.into(),
        subject: None,
        token: None,
        subscribe_url: None,
    };
    let canonical = canonical_message(&message).unwrap();
    let mut signer = Signer::new(MessageDigest::sha1(), &leaf_key).unwrap();
    signer.update(canonical.as_bytes()).unwrap();
    message.signature = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        signer.sign_to_vec().unwrap(),
    );
    let body = serde_json::to_vec(&serde_json::json!({
        "Type": message.message_type,
        "MessageId": message.message_id,
        "TopicArn": message.topic_arn,
        "Message": message.message,
        "Timestamp": message.timestamp,
        "SignatureVersion": message.signature_version,
        "Signature": message.signature,
        "SigningCertURL": message.signing_cert_url,
    }))
    .unwrap();
    verifier.verify(&body).await.expect("first verify");
    verifier.verify(&body).await.expect("cached verify");
}

async fn cleartext_verifier(topic: &str, authority_pem: Vec<u8>) -> WebhookVerifier {
    WebhookVerifier::new_allowing_cleartext(&WebhookTrustConfig {
        topic_arn: topic.to_string(),
        ca_bundle_pem: authority_pem,
        signing_certificate_host: "127.0.0.1".to_string(),
        confirmation_host: "127.0.0.1".to_string(),
    })
    .unwrap()
}

#[tokio::test]
async fn confirm_accepts_successful_and_rejects_failed_subscription_responses() {
    use axum::{Router, http::StatusCode, routing::get};
    use std::net::SocketAddr;

    let (authority, _) = certificate_authority("nvbes-test-ca");
    let verifier =
        cleartext_verifier("arn:scw:sns:fr-par:test:topic", authority.to_pem().unwrap()).await;

    let ok_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ok_addr = ok_listener.local_addr().unwrap();
    let ok_server = tokio::spawn(async move {
        axum::serve(
            ok_listener,
            Router::new().route("/confirm", get(|| async { StatusCode::NO_CONTENT })),
        )
        .await
        .unwrap();
    });
    let ok_url = format!("http://{ok_addr}/confirm").parse().unwrap();
    verifier.confirm(ok_url).await.expect("confirm succeeds");
    ok_server.abort();

    let fail_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let fail_addr = fail_listener.local_addr().unwrap();
    let fail_server = tokio::spawn(async move {
        axum::serve(
            fail_listener,
            Router::new().route("/confirm", get(|| async { StatusCode::FORBIDDEN })),
        )
        .await
        .unwrap();
    });
    let fail_url: reqwest::Url = format!("http://{fail_addr}/confirm").parse().unwrap();
    let err = verifier.confirm(fail_url).await.expect_err("rejected");
    assert!(
        err.to_string()
            .contains("SNS subscription confirmation was rejected"),
        "{err}"
    );
    fail_server.abort();

    let unreachable: SocketAddr = "127.0.0.1:1".parse().unwrap();
    let err = verifier
        .confirm(format!("http://{unreachable}/confirm").parse().unwrap())
        .await
        .expect_err("unreachable");
    assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn certificate_fetch_loads_valid_pem_and_rejects_oversized_or_failed_responses() {
    use axum::{
        Router,
        body::Body,
        http::{HeaderMap, HeaderValue, StatusCode, header},
        response::Response,
        routing::get,
    };

    let (authority, authority_key) = certificate_authority("nvbes-test-ca");
    let (leaf, _) = leaf_certificate("sns.scaleway.test", &authority, &authority_key);
    let pem = leaf.to_pem().unwrap();
    let verifier =
        cleartext_verifier("arn:scw:sns:fr-par:test:topic", authority.to_pem().unwrap()).await;

    let ok_pem = pem.clone();
    let ok_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ok_addr = ok_listener.local_addr().unwrap();
    let ok_server = tokio::spawn(async move {
        axum::serve(
            ok_listener,
            Router::new().route(
                "/cert.pem",
                get(move || {
                    let body = ok_pem.clone();
                    async move { Response::new(Body::from(body)) }
                }),
            ),
        )
        .await
        .unwrap();
    });
    let loaded = verifier
        .fetch_certificate_for_tests(&format!("http://{ok_addr}/cert.pem"))
        .await
        .expect("valid certificate");
    assert_eq!(loaded.to_pem().unwrap(), pem);
    // Second fetch hits the in-memory cache.
    verifier
        .fetch_certificate_for_tests(&format!("http://{ok_addr}/cert.pem"))
        .await
        .expect("cached certificate");
    ok_server.abort();

    let fail_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let fail_addr = fail_listener.local_addr().unwrap();
    let fail_server = tokio::spawn(async move {
        axum::serve(
            fail_listener,
            Router::new().route("/cert.pem", get(|| async { StatusCode::NOT_FOUND })),
        )
        .await
        .unwrap();
    });
    assert!(
        verifier
            .fetch_certificate_for_tests(&format!("http://{fail_addr}/cert.pem"))
            .await
            .is_err()
    );
    fail_server.abort();

    let oversized = vec![b'A'; 65 * 1024];
    let large_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let large_addr = large_listener.local_addr().unwrap();
    let large_server = tokio::spawn(async move {
        axum::serve(
            large_listener,
            Router::new().route(
                "/cert.pem",
                get(move || {
                    let body = oversized.clone();
                    async move {
                        let mut headers = HeaderMap::new();
                        headers.insert(header::CONTENT_LENGTH, HeaderValue::from_static("66560"));
                        (headers, Body::from(body))
                    }
                }),
            ),
        )
        .await
        .unwrap();
    });
    assert!(
        verifier
            .fetch_certificate_for_tests(&format!("http://{large_addr}/cert.pem"))
            .await
            .is_err()
    );
    large_server.abort();

    let body_too_large = vec![b'B'; 65 * 1024];
    let body_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let body_addr = body_listener.local_addr().unwrap();
    let body_server = tokio::spawn(async move {
        axum::serve(
            body_listener,
            Router::new().route(
                "/cert.pem",
                get(move || {
                    let body = body_too_large.clone();
                    async move { Response::new(Body::from(body)) }
                }),
            ),
        )
        .await
        .unwrap();
    });
    assert!(
        verifier
            .fetch_certificate_for_tests(&format!("http://{body_addr}/cert.pem"))
            .await
            .is_err()
    );
    body_server.abort();

    // Chunked body without Content-Length reaches the post-download size guard.
    let chunked = vec![b'C'; 65 * 1024];
    let chunk_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let chunk_addr = chunk_listener.local_addr().unwrap();
    let chunk_server = tokio::spawn(async move {
        axum::serve(
            chunk_listener,
            Router::new().route(
                "/cert.pem",
                get(move || {
                    let chunk = chunked.clone();
                    async move {
                        Body::from_stream(tokio_stream::once(Ok::<_, std::io::Error>(chunk)))
                    }
                }),
            ),
        )
        .await
        .unwrap();
    });
    let err = verifier
        .fetch_certificate_for_tests(&format!("http://{chunk_addr}/cert.pem"))
        .await
        .expect_err("chunked oversized certificate");
    assert!(
        err.to_string().contains("too large") || err.to_string().contains("could not be loaded"),
        "{err}"
    );
    chunk_server.abort();
}
