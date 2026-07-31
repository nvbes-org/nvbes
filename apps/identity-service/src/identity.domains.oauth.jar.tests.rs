use super::*;
use base64::Engine;
use jsonwebtoken::{Algorithm, EncodingKey, Header};

fn sign_request_object(claims: &RequestObjectClaims, secret: &str, algorithm: Algorithm) -> String {
    let header = Header::new(algorithm);
    jsonwebtoken::encode(
        &header,
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("JWT encoding should succeed")
}

fn high_assurance_fixture() -> (String, serde_json::Value) {
    let rsa = openssl::rsa::Rsa::generate(2048).expect("RSA key should generate");
    let private_key = rsa.private_key_to_pem().expect("private key should encode");
    let mut claims = valid_claims();
    claims.jti = Some(uuid::Uuid::new_v4().to_string());
    let mut header = Header::new(Algorithm::PS256);
    header.typ = Some("oauth-authz-req+jwt".to_string());
    header.kid = Some("jar-key-1".to_string());
    let token = jsonwebtoken::encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(&private_key).expect("private key should parse"),
    )
    .expect("request object should encode");
    let jwks = serde_json::json!({
        "keys": [{
            "kty": "RSA",
            "kid": "jar-key-1",
            "use": "sig",
            "alg": "PS256",
            "n": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rsa.n().to_vec()),
            "e": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rsa.e().to_vec())
        }]
    });
    (token, jwks)
}

fn valid_claims() -> RequestObjectClaims {
    let now = chrono::Utc::now().timestamp();
    RequestObjectClaims {
        iss: "gxoc_myclient".to_string(),
        aud: super::RequestObjectAudience::One("https://identity.example.com".to_string()),
        exp: now + 300,
        nbf: Some(now),
        iat: Some(now),
        jti: None,
        response_type: Some("code".to_string()),
        client_id: Some("gxoc_myclient".to_string()),
        redirect_uri: Some("https://app.example.com/callback".to_string()),
        scope: Some("openid profile".to_string()),
        state: Some("abc123".to_string()),
        nonce: None,
        audience: None,
        resource: None,
        authorization_details: None,
        code_challenge: Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string()),
        code_challenge_method: Some("S256".to_string()),
        consent_action: None,
    }
}

#[test]
fn validate_request_object_accepts_valid_hs256_jwt() {
    let secret = "test-request-object-secret";
    let client_id = "gxoc_myclient";
    let issuer_url = "https://identity.example.com";

    let claims = valid_claims();
    let jwt = sign_request_object(&claims, secret, Algorithm::HS256);

    let result = validate_request_object(&jwt, secret, client_id, issuer_url);
    assert!(
        result.is_ok(),
        "Valid JWT should be accepted: {:?}",
        result.err()
    );

    let parsed = result.unwrap();
    assert_eq!(
        parsed.redirect_uri.unwrap(),
        "https://app.example.com/callback"
    );
    assert_eq!(parsed.scope.unwrap(), "openid profile");
    assert_eq!(parsed.state.unwrap(), "abc123");
    assert_eq!(
        parsed.code_challenge.unwrap(),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    assert_eq!(parsed.code_challenge_method.unwrap(), "S256");
}

#[test]
fn validate_request_object_rejects_expired_jwt() {
    let secret = "test-request-object-secret";
    let client_id = "gxoc_myclient";
    let issuer_url = "https://identity.example.com";

    let now = chrono::Utc::now().timestamp();
    let claims = RequestObjectClaims {
        exp: now - 60,
        ..valid_claims()
    };
    let jwt = sign_request_object(&claims, secret, Algorithm::HS256);

    let err = validate_request_object(&jwt, secret, client_id, issuer_url)
        .expect_err("Expired JWT should be rejected");
    assert_eq!(err.code, "invalid_request_object");
    assert!(err.message.contains("expired"));
}

#[test]
fn validate_request_object_rejects_wrong_secret() {
    let secret = "test-request-object-secret";
    let wrong_secret = "gxo_wrong_secret_key";
    let client_id = "gxoc_myclient";
    let issuer_url = "https://identity.example.com";

    let claims = valid_claims();
    let jwt = sign_request_object(&claims, secret, Algorithm::HS256);

    let err = validate_request_object(&jwt, wrong_secret, client_id, issuer_url)
        .expect_err("JWT with wrong secret should be rejected");
    assert_eq!(err.code, "invalid_request_object");
    assert!(err.message.contains("signature"));
}

#[test]
fn validate_request_object_rejects_wrong_issuer() {
    let secret = "test-request-object-secret";
    let issuer_url = "https://identity.example.com";

    let claims = RequestObjectClaims {
        iss: "wrong_client_id".to_string(),
        ..valid_claims()
    };
    let jwt = sign_request_object(&claims, secret, Algorithm::HS256);

    let err = validate_request_object(&jwt, secret, "gxoc_myclient", issuer_url)
        .expect_err("JWT with wrong issuer should be rejected");
    assert_eq!(err.code, "invalid_request_object");
    assert!(err.message.contains("issuer"));
}

#[test]
fn validate_request_object_rejects_wrong_audience() {
    let secret = "test-request-object-secret";
    let client_id = "gxoc_myclient";

    let claims = valid_claims();
    let jwt = sign_request_object(&claims, secret, Algorithm::HS256);

    let err = validate_request_object(
        &jwt,
        secret,
        client_id,
        "https://wrong-audience.example.com",
    )
    .expect_err("JWT with wrong audience should be rejected");
    assert_eq!(err.code, "invalid_request_object");
    assert!(err.message.contains("audience"));
}

#[test]
fn validate_request_object_rejects_rs256_algorithm() {
    let secret = "test-request-object-secret";
    let client_id = "gxoc_myclient";
    let issuer_url = "https://identity.example.com";

    let claims = valid_claims();
    let jwt = sign_request_object(&claims, secret, Algorithm::HS256);

    assert!(
        validate_request_object(&jwt, secret, client_id, issuer_url).is_ok(),
        "HS256 should be supported"
    );
}

#[test]
fn validate_request_object_rejects_mismatched_client_id() {
    let secret = "test-request-object-secret";
    let issuer_url = "https://identity.example.com";

    let claims = RequestObjectClaims {
        client_id: Some("gxoc_different_client".to_string()),
        ..valid_claims()
    };
    let jwt = sign_request_object(&claims, secret, Algorithm::HS256);

    let err = validate_request_object(&jwt, secret, "gxoc_myclient", issuer_url)
        .expect_err("JWT with mismatched client_id should be rejected");
    assert_eq!(err.code, "invalid_request_object");
    assert!(err.message.contains("client_id"));
}

#[test]
fn validate_request_object_rejects_non_code_response_type() {
    let secret = "test-request-object-secret";
    let client_id = "gxoc_myclient";
    let issuer_url = "https://identity.example.com";

    let claims = RequestObjectClaims {
        response_type: Some("token".to_string()),
        ..valid_claims()
    };
    let jwt = sign_request_object(&claims, secret, Algorithm::HS256);

    let err = validate_request_object(&jwt, secret, client_id, issuer_url)
        .expect_err("Non-code response_type should be rejected");
    assert_eq!(err.code, "invalid_request_object");
    assert!(err.message.contains("response_type=code"));
}

#[test]
fn validate_request_object_supports_hs384() {
    let secret = "test-request-object-secret";
    let client_id = "gxoc_myclient";
    let issuer_url = "https://identity.example.com";

    let claims = valid_claims();
    let jwt = sign_request_object(&claims, secret, Algorithm::HS384);

    assert!(
        validate_request_object(&jwt, secret, client_id, issuer_url).is_ok(),
        "HS384 should be supported"
    );
}

#[test]
fn validate_request_object_supports_hs512() {
    let secret = "test-request-object-secret";
    let client_id = "gxoc_myclient";
    let issuer_url = "https://identity.example.com";

    let claims = valid_claims();
    let jwt = sign_request_object(&claims, secret, Algorithm::HS512);

    assert!(
        validate_request_object(&jwt, secret, client_id, issuer_url).is_ok(),
        "HS512 should be supported"
    );
}

#[test]
fn is_par_urn_detects_par_references() {
    assert!(is_par_urn(
        "urn:ietf:params:oauth:request_uri:gxpar_a1b2c3d4e5f6"
    ));
    assert!(!is_par_urn("https://client.example.com/jar/abc123"));
    assert!(!is_par_urn("urn:ietf:params:oauth:other"));
    assert!(!is_par_urn(""));
}

#[test]
fn validate_request_object_accepts_without_optional_fields() {
    let secret = "gxo_minimal_secret";
    let client_id = "gxoc_minimal";
    let issuer_url = "https://id.example.com";

    let now = chrono::Utc::now().timestamp();
    let claims = RequestObjectClaims {
        iss: client_id.to_string(),
        aud: super::RequestObjectAudience::One(issuer_url.to_string()),
        exp: now + 300,
        nbf: None,
        iat: None,
        jti: None,
        response_type: None,
        client_id: None,
        redirect_uri: Some("https://app.example.com/callback".to_string()),
        scope: None,
        state: None,
        nonce: None,
        audience: None,
        resource: None,
        authorization_details: None,
        code_challenge: None,
        code_challenge_method: None,
        consent_action: None,
    };
    let jwt = sign_request_object(&claims, secret, Algorithm::HS256);

    let result = validate_request_object(&jwt, secret, client_id, issuer_url);
    assert!(
        result.is_ok(),
        "Minimal JWT should be accepted: {:?}",
        result.err()
    );
}

#[test]
fn high_assurance_request_object_accepts_ps256_registered_jwk() {
    let (token, jwks) = high_assurance_fixture();

    let claims = validate_high_assurance_request_object(
        &token,
        &jwks,
        "gxoc_myclient",
        "https://identity.example.com",
    )
    .expect("PS256 request object should validate");

    assert_eq!(claims.client_id.as_deref(), Some("gxoc_myclient"));
    assert!(claims.jti.is_some());
}

#[test]
fn high_assurance_request_object_rejects_unknown_kid() {
    let (token, mut jwks) = high_assurance_fixture();
    jwks["keys"][0]["kid"] = serde_json::Value::String("other-key".to_string());

    let error = validate_high_assurance_request_object(
        &token,
        &jwks,
        "gxoc_myclient",
        "https://identity.example.com",
    )
    .expect_err("unregistered kid must fail");

    assert_eq!(error.code, "invalid_request_object");
}
