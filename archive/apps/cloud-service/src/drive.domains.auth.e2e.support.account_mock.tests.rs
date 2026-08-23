use super::{
    MockSigningKey, TEST_CLOUD_AUDIENCE, TestTokenClaims, decode_test_claims, sign_claims,
};
use axum::http::StatusCode;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn token_decoding_is_bound_to_the_mock_server_issuer() {
    let key = MockSigningKey::generate();
    let issuer = "http://127.0.0.1:43123";
    let now = Utc::now().timestamp();
    let claims = TestTokenClaims {
        sub: Uuid::new_v4().to_string(),
        workspace_id: Some(Uuid::new_v4().to_string()),
        tenant_id: Some(Uuid::new_v4().to_string()),
        organization_id: None,
        token_type: "access".to_string(),
        scope: "drive.workspace.read".to_string(),
        role: "owner".to_string(),
        amr: vec!["m2m".to_string()],
        client_id: Some("cloud-test-client".to_string()),
        iss: issuer.to_string(),
        aud: TEST_CLOUD_AUDIENCE.to_string(),
        exp: now + 60,
        iat: now,
        nbf: now,
        jti: Uuid::new_v4().to_string(),
        sid: Uuid::new_v4().to_string(),
    };
    let token = sign_claims(&key, &claims).expect("test token should be signed");

    let decoded =
        decode_test_claims(&key, issuer, &token).expect("matching issuer should be accepted");
    assert_eq!(decoded.iss, issuer);
    assert_eq!(
        decode_test_claims(&key, "http://127.0.0.1:1", &token)
            .expect_err("another issuer must be rejected"),
        StatusCode::UNAUTHORIZED
    );
}
