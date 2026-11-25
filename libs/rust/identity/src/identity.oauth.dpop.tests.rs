use super::verify_code_proof;
use nvbes_dpop::{generate_key_pair, proof::create_dpop_proof};
use rsa::pkcs8::EncodePrivateKey;

fn signed_claims(iat: i64, jti: &str) -> String {
    let pair = generate_key_pair();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
    header.typ = Some("dpop+jwt".into());
    header.jwk = Some(serde_json::from_value(serde_json::to_value(&pair.jwk).unwrap()).unwrap());
    let key = pair.secret_key.to_pkcs8_der().unwrap();
    jsonwebtoken::encode(
        &header,
        &serde_json::json!({
            "jti": jti, "iat": iat, "htm": "POST", "htu": "https://identity.example/oauth/token"
        }),
        &jsonwebtoken::EncodingKey::from_ec_der(key.as_bytes()),
    )
    .unwrap()
}

#[test]
fn replay_retention_covers_the_entire_future_dated_proof_window() {
    let iat = chrono::Utc::now().timestamp() + 240;
    let proof = signed_claims(iat, "future-proof");
    let verified =
        super::verify_token_proof(&proof, "POST", "https://identity.example/oauth/token").unwrap();
    assert_eq!(verified.expires_at.timestamp(), iat + 301);
    assert!(verified.expires_at > chrono::Utc::now() + chrono::Duration::seconds(500));
}

#[test]
fn token_proofs_reject_empty_identifiers_and_out_of_window_timestamps() {
    let now = chrono::Utc::now().timestamp();
    for (iat, jti) in [(now, ""), (now - 301, "expired"), (now + 600, "future")] {
        let proof = signed_claims(iat, jti);
        assert!(
            super::verify_token_proof(&proof, "POST", "https://identity.example/oauth/token")
                .is_err()
        );
    }
}

#[test]
fn code_proof_returns_only_thumbprint_after_cryptographic_verification() {
    let pair = generate_key_pair();
    let proof = create_dpop_proof(
        &pair,
        "POST",
        "https://identity.example/oauth/token",
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        verify_code_proof(&proof, "POST", "https://identity.example/oauth/token").unwrap(),
        pair.jkt
    );
    assert!(verify_code_proof(&proof, "GET", "https://identity.example/oauth/token").is_err());
}
