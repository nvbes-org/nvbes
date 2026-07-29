use super::is_supported_client_assertion_public_jwk;
use super::jwk::{algorithm_from_name, supported_algorithm};
use jsonwebtoken::Algorithm;

#[test]
fn algorithm_policy_accepts_private_key_jwt_algorithms_only() {
    assert!(supported_algorithm(Algorithm::RS256).is_ok());
    assert!(supported_algorithm(Algorithm::ES256).is_ok());
    assert!(supported_algorithm(Algorithm::EdDSA).is_ok());
    assert!(supported_algorithm(Algorithm::HS256).is_err());
}

#[test]
fn algorithm_names_are_strict() {
    assert_eq!(algorithm_from_name("RS256"), Some(Algorithm::RS256));
    assert_eq!(algorithm_from_name("none"), None);
}

#[test]
fn registered_public_jwk_requires_supported_key_shape() {
    assert!(!is_supported_client_assertion_public_jwk(
        &serde_json::json!({"kty": "oct", "alg": "HS256", "k": "secret"})
    ));
}
