use super::verify_code_proof;
use nvbes_dpop::{generate_key_pair, proof::create_dpop_proof};

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
