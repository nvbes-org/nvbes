use openssl::{hash::MessageDigest, sign::Signer};

use super::{
    MAX_MESSAGE_AGE, SnsMessage, WebhookVerifier, canonical_message, sns_timestamp_within_window,
    validated_url,
};
use crate::config::WebhookTrustConfig;
use crate::webhook_verify_fixtures::{certificate_authority, leaf_certificate};

#[test]
fn sns_timestamp_window_accepts_exact_bounds_and_rejects_beyond() {
    let now = chrono::Utc::now();
    assert!(sns_timestamp_within_window(now, now, MAX_MESSAGE_AGE));
    assert!(sns_timestamp_within_window(
        now - MAX_MESSAGE_AGE,
        now,
        MAX_MESSAGE_AGE
    ));
    assert!(sns_timestamp_within_window(
        now + MAX_MESSAGE_AGE,
        now,
        MAX_MESSAGE_AGE
    ));
    assert!(!sns_timestamp_within_window(
        now - MAX_MESSAGE_AGE - chrono::Duration::nanoseconds(1),
        now,
        MAX_MESSAGE_AGE
    ));
    assert!(!sns_timestamp_within_window(
        now + MAX_MESSAGE_AGE + chrono::Duration::nanoseconds(1),
        now,
        MAX_MESSAGE_AGE
    ));
}

#[test]
fn webhook_verifier_rejects_an_empty_ca_bundle() {
    assert!(
        WebhookVerifier::new(&WebhookTrustConfig {
            topic_arn: "arn:scw:sns:fr-par:test:topic".to_string(),
            ca_bundle_pem: Vec::new(),
            signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
            confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
        })
        .is_err()
    );
    assert!(
        WebhookVerifier::new_allowing_cleartext(&WebhookTrustConfig {
            topic_arn: "arn:scw:sns:fr-par:test:topic".to_string(),
            ca_bundle_pem: Vec::new(),
            signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
            confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
        })
        .is_err()
    );
}

#[test]
fn signing_certificate_url_is_strictly_allowlisted() {
    assert!(
        validated_url(
            "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem",
            "messaging.s3.fr-par.scw.cloud",
            "/fr-par/sns/"
        )
        .is_ok()
    );
    assert!(
        validated_url(
            "https://evil.example/fr-par/sns/cert.pem",
            "messaging.s3.fr-par.scw.cloud",
            "/fr-par/sns/"
        )
        .is_err()
    );
    assert!(
        validated_url(
            "https://messaging.s3.fr-par.scw.cloud.evil.example/fr-par/sns/cert.pem",
            "messaging.s3.fr-par.scw.cloud",
            "/fr-par/sns/"
        )
        .is_err()
    );
}

#[test]
fn notification_canonical_form_matches_sns_v1_field_order() {
    let message = SnsMessage {
        message_type: "Notification".into(),
        message_id: "id".into(),
        topic_arn: "arn".into(),
        message: "payload".into(),
        timestamp: "2026-08-02T12:00:00Z".into(),
        signature_version: "1".into(),
        signature: "signature".into(),
        signing_cert_url: "https://example.test/cert.pem".into(),
        subject: None,
        token: None,
        subscribe_url: None,
    };
    assert_eq!(
        canonical_message(&message).unwrap(),
        "Message\npayload\nMessageId\nid\nTimestamp\n2026-08-02T12:00:00Z\nTopicArn\narn\nType\nNotification\n"
    );
}

#[test]
fn subscription_envelope_and_confirmation_url_are_strictly_validated() {
    let (authority, _) = certificate_authority("nvbes-test-ca");
    let verifier = WebhookVerifier::new(&WebhookTrustConfig {
        topic_arn: "arn:scw:sns:fr-par:test:topic".to_string(),
        ca_bundle_pem: authority.to_pem().unwrap(),
        signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
        confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
    })
    .unwrap();
    let token = "confirmation-token";
    let topic = "arn:scw:sns:fr-par:test:topic";
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
            "https://sns.mnq.fr-par.scaleway.com/?Action=ConfirmSubscription&TopicArn={topic}&Token={token}"
        )),
    };
    assert!(verifier.validate_envelope(&message).is_ok());
    assert!(verifier.confirmation_url(&message).is_ok());
    assert!(
        canonical_message(&message)
            .unwrap()
            .contains("SubscribeURL")
    );

    message.signature_version = "2".into();
    assert!(verifier.validate_envelope(&message).is_err());
    message.signature_version = "1".into();
    message.topic_arn = "other".into();
    assert!(verifier.validate_envelope(&message).is_err());
    message.topic_arn = topic.into();
    message.message_type = "Unknown".into();
    assert!(verifier.validate_envelope(&message).is_err());
    message.message_type = "SubscriptionConfirmation".into();
    message.timestamp = (chrono::Utc::now() - chrono::Duration::minutes(6)).to_rfc3339();
    assert!(verifier.validate_envelope(&message).is_err());
    // Live-clock checks stay slightly inside the window to avoid race flakes.
    message.timestamp = (chrono::Utc::now() - chrono::Duration::minutes(5)
        + chrono::Duration::milliseconds(250))
    .to_rfc3339();
    assert!(verifier.validate_envelope(&message).is_ok());
    message.timestamp = (chrono::Utc::now() + chrono::Duration::minutes(5)
        - chrono::Duration::milliseconds(250))
    .to_rfc3339();
    assert!(verifier.validate_envelope(&message).is_ok());
    message.timestamp = chrono::Utc::now().to_rfc3339();
    message.subscribe_url = Some(format!(
        "https://user:secret@sns.mnq.fr-par.scaleway.com/?Action=ConfirmSubscription&TopicArn={topic}&Token={token}"
    ));
    assert!(
        verifier.confirmation_url(&message).is_err(),
        "confirmation URLs with credentials must be rejected"
    );
    message.subscribe_url = None;
    assert!(verifier.confirmation_url(&message).is_err());
    assert!(canonical_message(&message).is_err());
}

#[tokio::test]
async fn verification_rejects_a_valid_signature_when_certificate_fetch_fails() {
    let (authority, authority_key) = certificate_authority("nvbes-test-ca");
    let (_leaf, leaf_key) = leaf_certificate("sns.scaleway.test", &authority, &authority_key);
    let topic = "arn:scw:sns:fr-par:test:topic";
    let payload = r#"{"id":"provider-event-1","email_id":"unknown","type":"delivery"}"#;
    let mut message = SnsMessage {
        message_type: "Notification".into(),
        message_id: "message-id".into(),
        topic_arn: topic.into(),
        message: payload.into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        signature_version: "1".into(),
        signature: String::new(),
        signing_cert_url: "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem".into(),
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

    let verifier = WebhookVerifier::new(&WebhookTrustConfig {
        topic_arn: topic.to_string(),
        ca_bundle_pem: authority.to_pem().unwrap(),
        signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
        confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
    })
    .unwrap();
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
    let error = verifier
        .verify(&body)
        .await
        .expect_err("certificate fetch fails");
    assert_eq!(error.outcome(), "invalid_certificate");
}

#[tokio::test]
async fn verification_rejects_malformed_and_untrusted_envelopes_before_network_io() {
    let (authority, _) = certificate_authority("nvbes-test-ca");
    let verifier = WebhookVerifier::new(&WebhookTrustConfig {
        topic_arn: "arn:scw:sns:fr-par:test:topic".to_string(),
        ca_bundle_pem: authority.to_pem().unwrap(),
        signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
        confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
    })
    .unwrap();
    assert!(verifier.verify(b"not-json").await.is_err());
    let body = serde_json::json!({
        "Type": "Notification",
        "MessageId": "id",
        "TopicArn": "wrong-topic",
        "Message": "{}",
        "Timestamp": chrono::Utc::now().to_rfc3339(),
        "SignatureVersion": "1",
        "Signature": "invalid",
        "SigningCertURL": "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem"
    });
    assert!(
        verifier
            .verify(&serde_json::to_vec(&body).unwrap())
            .await
            .is_err()
    );
}

#[test]
fn url_allowlists_reject_credentials_ports_paths_and_queries() {
    let host = "messaging.s3.fr-par.scw.cloud";
    for value in [
        "http://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem",
        "https://user@messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem",
        "https://messaging.s3.fr-par.scw.cloud:444/fr-par/sns/cert.pem",
        "https://messaging.s3.fr-par.scw.cloud/other/cert.pem",
        "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem?x=1",
        "https://messaging.s3.fr-par.scw.cloud/fr-par/sns/cert.pem#fragment",
    ] {
        assert!(validated_url(value, host, "/fr-par/sns/").is_err());
    }
}

#[test]
fn signing_certificate_must_chain_to_the_pinned_authority() {
    let (authority, authority_key) = certificate_authority("nvbes-test-ca");
    let (trusted_leaf, _) = leaf_certificate("sns.scaleway.test", &authority, &authority_key);
    let (other_authority, other_key) = certificate_authority("untrusted-ca");
    let (untrusted_leaf, _) = leaf_certificate("sns.scaleway.test", &other_authority, &other_key);
    let verifier = WebhookVerifier::new(&WebhookTrustConfig {
        topic_arn: "arn:scw:sns:fr-par:test:topic".to_string(),
        ca_bundle_pem: authority.to_pem().unwrap(),
        signing_certificate_host: "messaging.s3.fr-par.scw.cloud".to_string(),
        confirmation_host: "sns.mnq.fr-par.scaleway.com".to_string(),
    })
    .unwrap();

    assert!(verifier.validate_certificate(&trusted_leaf).is_ok());
    assert!(verifier.validate_certificate(&untrusted_leaf).is_err());
}

#[test]
fn verification_error_outcomes_are_stable() {
    use super::SnsVerificationError;
    assert_eq!(
        SnsVerificationError::Envelope(anyhow::anyhow!("bad")).outcome(),
        "invalid_envelope"
    );
    assert_eq!(
        SnsVerificationError::CertificateUrl(anyhow::anyhow!("bad")).outcome(),
        "invalid_certificate_url"
    );
    assert_eq!(
        SnsVerificationError::Certificate(anyhow::anyhow!("bad")).outcome(),
        "invalid_certificate"
    );
    assert_eq!(
        SnsVerificationError::SignatureEncoding(
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b"!!!!")
                .expect_err("invalid base64")
        )
        .outcome(),
        "invalid_signature_encoding"
    );
    assert_eq!(
        SnsVerificationError::Cryptography(openssl::error::ErrorStack::get()).outcome(),
        "cryptography_failed"
    );
    assert_eq!(
        SnsVerificationError::InvalidSignature.outcome(),
        "invalid_signature"
    );
}

#[test]
fn notification_canonical_form_includes_optional_subject() {
    let message = SnsMessage {
        message_type: "Notification".into(),
        message_id: "id".into(),
        topic_arn: "arn".into(),
        message: "payload".into(),
        timestamp: "2026-08-02T12:00:00Z".into(),
        signature_version: "1".into(),
        signature: "signature".into(),
        signing_cert_url: "https://example.test/cert.pem".into(),
        subject: Some("hello".into()),
        token: None,
        subscribe_url: None,
    };
    assert!(
        canonical_message(&message)
            .unwrap()
            .contains("Subject\nhello\n")
    );
}

#[test]
fn canonical_message_rejects_unsupported_type() {
    let message = SnsMessage {
        message_type: "UnsubscribeConfirmation".into(),
        message_id: "id".into(),
        topic_arn: "arn".into(),
        message: "payload".into(),
        timestamp: "2026-08-02T12:00:00Z".into(),
        signature_version: "1".into(),
        signature: "signature".into(),
        signing_cert_url: "https://example.test/cert.pem".into(),
        subject: None,
        token: None,
        subscribe_url: None,
    };
    assert!(canonical_message(&message).is_err());
}
