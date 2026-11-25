use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;

use super::{
    DEFAULT_OPERATOR_ROLE, MFA_STEP_UP_HEADER, OperatorAuthPolicy, OperatorSession, Permission,
    require_operator_auth,
};

#[derive(Serialize)]
struct TestClaims {
    sub: String,
    role: String,
    amr: Vec<String>,
    auth_time: i64,
    exp: i64,
    nbf: i64,
    iss: String,
    aud: String,
}

fn encode_token(claims: TestClaims) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(b"test-signing-key-for-platform-operations"),
    )
    .expect("token must encode")
}

fn auth_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).expect("header"),
    );
    headers
}

fn valid_claims(auth_age_secs: i64) -> TestClaims {
    let now = Utc::now().timestamp();
    TestClaims {
        sub: "operator-1".to_string(),
        role: DEFAULT_OPERATOR_ROLE.to_string(),
        amr: vec!["pwd".to_string(), "mfa".to_string()],
        auth_time: now - auth_age_secs,
        exp: now + 600,
        nbf: now - 1,
        iss: "test-identity".to_string(),
        aud: "platform-operations".to_string(),
    }
}

#[test]
fn operator_session_only_permits_platform_owner() {
    let owner = OperatorSession {
        operator_id: "op".into(),
        role: DEFAULT_OPERATOR_ROLE.into(),
        has_mfa_step_up: true,
    };
    assert!(owner.permits(Permission::ReadCases));
    assert!(owner.permits(Permission::WriteCosts));

    let other = OperatorSession {
        operator_id: "op".into(),
        role: "viewer".into(),
        has_mfa_step_up: true,
    };
    assert!(!other.permits(Permission::ReadAudit));
}

#[test]
fn authenticate_headers_accepts_fresh_mfa_session() {
    let policy = OperatorAuthPolicy::test_policy();
    let token = encode_token(valid_claims(30));
    let session = policy
        .authenticate_headers(&auth_headers(&token))
        .expect("fresh MFA session must pass");
    assert_eq!(session.operator_id, "operator-1");
    assert!(session.has_mfa_step_up);
    assert_eq!(session.role, DEFAULT_OPERATOR_ROLE);
}

#[test]
fn authenticate_headers_marks_stale_mfa_without_step_up() {
    let policy = OperatorAuthPolicy::test_policy();
    let token = encode_token(valid_claims(301));
    let session = policy
        .authenticate_headers(&auth_headers(&token))
        .expect("stale MFA still authenticates");
    assert!(!session.has_mfa_step_up);
}

#[test]
fn authenticate_headers_rejects_missing_bearer_wrong_role_and_missing_mfa() {
    let policy = OperatorAuthPolicy::test_policy();
    assert_eq!(
        policy.authenticate_headers(&HeaderMap::new()),
        Err(StatusCode::UNAUTHORIZED)
    );

    let mut claims = valid_claims(10);
    claims.role = "viewer".to_string();
    assert_eq!(
        policy.authenticate_headers(&auth_headers(&encode_token(claims))),
        Err(StatusCode::FORBIDDEN)
    );

    let mut claims = valid_claims(10);
    claims.amr = vec!["pwd".to_string()];
    assert_eq!(
        policy.authenticate_headers(&auth_headers(&encode_token(claims))),
        Err(StatusCode::FORBIDDEN)
    );

    let mut claims = valid_claims(10);
    claims.sub = " ".to_string();
    assert_eq!(
        policy.authenticate_headers(&auth_headers(&encode_token(claims))),
        Err(StatusCode::FORBIDDEN)
    );
}

#[tokio::test]
async fn require_operator_auth_maps_status_to_json_error() {
    let policy = OperatorAuthPolicy::test_policy();
    let err = require_operator_auth(&policy, &HeaderMap::new())
        .await
        .expect_err("missing auth must fail");
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        err.1.get("error"),
        Some(&serde_json::json!("operator_authentication_required"))
    );
}

#[test]
fn mfa_step_up_header_constant_is_stable() {
    assert_eq!(MFA_STEP_UP_HEADER, "x-nvbes-mfa-step-up");
}

#[test]
fn from_rsa_pem_rejects_invalid_pem_and_accepts_valid_key() {
    assert!(
        OperatorAuthPolicy::from_rsa_pem(b"not-a-pem", "issuer", "platform-operations").is_err()
    );

    // Minimal RSA public key for DecodingKey::from_rsa_pem coverage only.
    const RSA_PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA7PFS2iyDkuP51I+gvRSo
vf5kUCTMlqT0jqKh0b6swAE5gKc/4U5+momWcSqNStAcfNep9LcuWUb6Gn6WnE01
SOPVQb72JpmCR7iFcddQGgYAQNmO8k0wL1DlidjZd+r2ENbANxpkgyhFXER7OzDm
yUO5B6hMqzWtua/m97vaeWzpYmnaqmitWwhrKpLS7P0YAgOLCfEBkEM1UwpTX0k5
UTNfigLIL5NdCSNyFGhGeFUfh1JVoaWhFlsVGK7ht0+y1TEuu+//fVfqHWOR0ptk
HMbCh1dryfv3cpkQEktjU/JWkj41ouPPw3bZiGiW9AlRFDLOxXQz6dVutGJ5wviC
EwIDAQAB
-----END PUBLIC KEY-----";
    let policy = OperatorAuthPolicy::from_rsa_pem(
        RSA_PUBLIC_PEM.as_bytes(),
        "test-identity",
        "platform-operations",
    )
    .expect("valid rsa pem must load");
    assert_eq!(
        policy.authenticate_headers(&HeaderMap::new()),
        Err(StatusCode::UNAUTHORIZED)
    );
}

#[test]
fn authenticate_headers_accepts_webauthn_amr_and_rejects_oversized_sub() {
    let policy = OperatorAuthPolicy::test_policy();
    let mut claims = valid_claims(10);
    claims.amr = vec!["webauthn".to_string()];
    let session = policy
        .authenticate_headers(&auth_headers(&encode_token(claims)))
        .expect("webauthn counts as MFA");
    assert!(session.has_mfa_step_up);

    let mut claims = valid_claims(10);
    claims.sub = "x".repeat(129);
    assert_eq!(
        policy.authenticate_headers(&auth_headers(&encode_token(claims))),
        Err(StatusCode::FORBIDDEN)
    );
}
