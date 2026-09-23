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

    message.timestamp = (chrono::Utc::now() + chrono::Duration::minutes(6)).to_rfc3339();
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
