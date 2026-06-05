use super::*;
use axum::http::{HeaderMap, HeaderValue};
use ed25519_dalek::{Signer, SigningKey};

#[test]
fn verify_accepts_valid_ed25519_signature() {
    let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
    let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());
    let created = Utc::now().timestamp();
    let signature_input = format!(
        "sig1=(\"@method\" \"@target-uri\" \"content-digest\");keyid=\"gx_test_signed\";created={created}"
    );

    let mut headers = HeaderMap::new();
    headers.insert("host", HeaderValue::from_static("api.nvbes.test"));
    headers.insert("content-digest", HeaderValue::from_static("sha-256=:abc=:"));
    headers.insert(
        "signature-input",
        HeaderValue::from_str(&signature_input).unwrap(),
    );

    let uri = Uri::from_static("/v1/workspaces/00000000-0000-0000-0000-000000000000/uploads");
    let params = parse_signature_input(&signature_input).unwrap();
    let base = signature_base(&headers, &Method::POST, &uri, &params).unwrap();
    let signature = signing_key.sign(base.as_bytes());
    let signature_header = format!("sig1=:{}:", STANDARD.encode(signature.to_bytes()));
    headers.insert(
        "signature",
        HeaderValue::from_str(&signature_header).unwrap(),
    );

    verify(&headers, &Method::POST, &uri, &public_key).unwrap();
}

#[test]
fn post_signatures_must_cover_content_digest() {
    let params = parse_signature_input(
        "sig1=(\"@method\" \"@target-uri\");keyid=\"gx_test_signed\";created=1893456000",
    )
    .unwrap();

    assert!(validate_required_components(&params, &Method::POST).is_err());
}
