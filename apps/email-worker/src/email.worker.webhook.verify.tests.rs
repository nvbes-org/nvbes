use openssl::{
    asn1::Asn1Time,
    bn::{BigNum, MsbOption},
    hash::MessageDigest,
    pkey::{PKey, Private},
    rsa::Rsa,
    x509::{X509, X509NameBuilder, extension::BasicConstraints},
};

use super::{SnsMessage, WebhookVerifier, canonical_message, validated_url};
use crate::config::WebhookTrustConfig;

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
fn signing_certificate_must_chain_to_the_pinned_authority() {
    let (authority, authority_key) = certificate_authority("nvbes-test-ca");
    let trusted_leaf = leaf_certificate("sns.scaleway.test", &authority, &authority_key);
    let (other_authority, other_key) = certificate_authority("untrusted-ca");
    let untrusted_leaf = leaf_certificate("sns.scaleway.test", &other_authority, &other_key);
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

fn certificate_authority(common_name: &str) -> (X509, PKey<Private>) {
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let name = name(common_name);
    let mut builder = base_certificate(&name, &name, &key);
    builder
        .append_extension(BasicConstraints::new().critical().ca().build().unwrap())
        .unwrap();
    builder.sign(&key, MessageDigest::sha256()).unwrap();
    (builder.build(), key)
}

fn leaf_certificate(common_name: &str, issuer: &X509, issuer_key: &PKey<Private>) -> X509 {
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let subject = name(common_name);
    let mut builder = base_certificate(&subject, issuer.subject_name(), &key);
    builder.sign(issuer_key, MessageDigest::sha256()).unwrap();
    builder.build()
}

fn base_certificate(
    subject: &openssl::x509::X509NameRef,
    issuer: &openssl::x509::X509NameRef,
    key: &PKey<Private>,
) -> openssl::x509::X509Builder {
    let mut builder = X509::builder().unwrap();
    let mut serial = BigNum::new().unwrap();
    serial.rand(128, MsbOption::MAYBE_ZERO, false).unwrap();
    builder
        .set_serial_number(&serial.to_asn1_integer().unwrap())
        .unwrap();
    builder.set_version(2).unwrap();
    builder.set_subject_name(subject).unwrap();
    builder.set_issuer_name(issuer).unwrap();
    builder.set_pubkey(key).unwrap();
    builder
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::days_from_now(30).unwrap())
        .unwrap();
    builder
}

fn name(common_name: &str) -> openssl::x509::X509Name {
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", common_name).unwrap();
    name.build()
}
