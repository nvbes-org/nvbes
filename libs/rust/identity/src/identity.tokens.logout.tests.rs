use super::*;
use crate::tokens::tests::config;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::{Value, json};

fn claims() -> Value {
    let now = Utc::now().timestamp() as u64;
    json!({
        "iss":config().issuer,"aud":"account-web","sub":Uuid::new_v4(),
        "sid":Uuid::new_v4(),"iat":now-1000,"exp":now-100,"auth_time":now-1100,
        "amr":["pwd"],"nonce":"original-nonce","at_hash":"original-access-hash"
    })
}

fn header() -> Header {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(config().key_id);
    header
}

fn sign(header: &Header, claims: &Value) -> String {
    encode(
        header,
        claims,
        &EncodingKey::from_rsa_pem(config().private_key_pem.as_bytes()).unwrap(),
    )
    .unwrap()
}

#[test]
fn expired_id_token_is_only_a_hint_and_cannot_authorize_an_api() {
    let service = TokenService::new(config()).unwrap();
    let claims = claims();
    let token = sign(&header(), &claims);
    let hint = service.verify_logout_hint(&token).unwrap();
    assert_eq!(hint.client_id, "account-web");
    assert_eq!(hint.session_id.to_string(), claims["sid"].as_str().unwrap());
    assert_eq!(
        hint.principal_id.to_string(),
        claims["sub"].as_str().unwrap()
    );
    assert_eq!(hint.issued_at, claims["iat"].as_u64().unwrap());
    assert!(service.verify(&token, "nvbes-account-service").is_err());
}

#[test]
fn wrong_issuer_malformed_identity_and_impossible_times_are_rejected() {
    let service = TokenService::new(config()).unwrap();
    let now = Utc::now().timestamp() as u64;
    for (field, value) in [
        ("iss", json!("https://other.example")),
        ("sub", json!("invalid")),
        ("sid", json!("invalid")),
        ("aud", json!("")),
        ("aud", json!(["account-web", "another-client"])),
        ("iat", json!(now + 10)),
        ("auth_time", json!(now + 10)),
        ("exp", json!(now - 1001)),
        ("exp", json!(now + 1000)),
        ("amr", json!(["invented"])),
    ] {
        let mut claims = claims();
        claims[field] = value;
        assert!(
            service
                .verify_logout_hint(&sign(&header(), &claims))
                .is_err(),
            "{field}"
        );
    }
    for field in ["iss", "aud", "sub", "sid", "iat", "exp", "auth_time", "amr"] {
        let mut claims = claims();
        claims.as_object_mut().unwrap().remove(field);
        assert!(
            service
                .verify_logout_hint(&sign(&header(), &claims))
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn token_type_key_selection_and_remote_key_headers_cannot_be_substituted() {
    let service = TokenService::new(config()).unwrap();
    let mut wrong_type = header();
    wrong_type.typ = Some("at+jwt".into());
    let mut no_type = header();
    no_type.typ = None;
    let mut unknown_key = header();
    unknown_key.kid = Some("unknown".into());
    let mut no_key = header();
    no_key.kid = None;
    let mut jku = header();
    jku.jku = Some("https://attacker.example/keys".into());
    let mut x5u = header();
    x5u.x5u = Some("https://attacker.example/cert".into());
    for header in [wrong_type, no_type, unknown_key, no_key, jku, x5u] {
        assert!(
            service
                .verify_logout_hint(&sign(&header, &claims()))
                .is_err()
        );
    }
    let token = sign(&header(), &claims());
    let mut segments: Vec<_> = token.split('.').collect();
    segments[2] = "invalid-signature";
    assert!(service.verify_logout_hint(&segments.join(".")).is_err());
    assert!(service.verify_logout_hint(&"a".repeat(16_385)).is_err());
}

#[test]
fn expired_hint_does_not_resurrect_a_retired_signing_key() {
    let old = config();
    let token = sign(&header(), &claims());
    let mut next = old.clone();
    next.key_id = "new-key".into();
    let overlap = json!([{"kid":old.key_id,"public_key_pem":old.public_key_pem,"accept_until":1}]);
    let service =
        TokenService::new(next.with_verification_keys(&overlap.to_string()).unwrap()).unwrap();
    assert!(service.verify_logout_hint(&token).is_err());
}
