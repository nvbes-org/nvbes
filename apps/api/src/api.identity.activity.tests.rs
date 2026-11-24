use super::*;
use nvbes_identity_service::tokens_claims::ProofConfirmation;

fn claims(cnf: Option<ProofConfirmation>) -> AccessTokenClaims {
    AccessTokenClaims {
        sub: "00000000-0000-4000-8000-000000000001".into(),
        token_type: "access".into(),
        client_id: "nvbes-account-web".into(),
        scope: "account:read".into(),
        amr: vec!["pwd".into()],
        auth_time: 10,
        step_up_time: None,
        step_up_expires_at: None,
        iss: "http://localhost:8080".into(),
        aud: "nvbes-account-service".into(),
        exp: u64::MAX,
        iat: 10,
        nbf: 10,
        jti: "00000000-0000-4000-8000-000000000002".into(),
        sid: "00000000-0000-4000-8000-000000000003".into(),
        grant_id: "00000000-0000-4000-8000-000000000004".into(),
        cnf,
    }
}

#[test]
fn bearer_claims_are_presented_like_the_introspection_endpoint() {
    let presented = presented(claims(None)).unwrap();
    assert_eq!(presented.token_type, "Bearer");
    assert_eq!(presented.cnf, None);
    assert_eq!(presented.aud, "nvbes-account-service");
    assert_eq!(presented.grant_id, "00000000-0000-4000-8000-000000000004");
}

#[test]
fn dpop_claims_carry_their_confirmation() {
    let presented = presented(claims(Some(ProofConfirmation {
        jkt: "thumbprint".into(),
    })))
    .unwrap();
    assert_eq!(presented.token_type, "DPoP");
    assert_eq!(
        presented.cnf,
        Some(serde_json::json!({"jkt": "thumbprint"}))
    );
}

#[test]
fn a_diverging_authority_answer_is_unavailable_not_inactive() {
    let actual = presented(claims(None)).unwrap();
    let mut expected = actual.clone();
    expected.scope = "account:write".into();
    assert!(matches!(
        confirm_activity(&actual, &expected),
        Err(IntrospectionError::Unavailable)
    ));
    assert!(matches!(confirm_activity(&actual, &actual), Ok(true)));
}
