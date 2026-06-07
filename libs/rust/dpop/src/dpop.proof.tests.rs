use crate::keys::generate_key_pair;

use super::*;

#[test]
fn create_and_verify_dpop_proof_roundtrip() {
    let pair = generate_key_pair();
    let proof = create_dpop_proof(&pair, "GET", "https://api.example.com/resource", None, None)
        .expect("proof creation should succeed");

    let verified = verify_dpop_proof(&proof, "GET", "https://api.example.com/resource", None, 300)
        .expect("verification should succeed");

    assert_eq!(verified.claims.htm, "GET");
    assert_eq!(verified.claims.htu, "https://api.example.com/resource");
}

#[test]
fn dpop_proof_with_ath_binds_to_token() {
    let pair = generate_key_pair();
    let token = "my-access-token-value";
    let proof = create_dpop_proof(
        &pair,
        "POST",
        "https://api.example.com/data",
        Some(token),
        None,
    )
    .expect("proof creation should succeed");

    verify_dpop_proof(
        &proof,
        "POST",
        "https://api.example.com/data",
        Some(token),
        300,
    )
    .expect("verification with correct ath should succeed");
}

#[test]
fn dpop_proof_rejects_wrong_ath() {
    let pair = generate_key_pair();
    let proof = create_dpop_proof(
        &pair,
        "GET",
        "https://api.example.com/resource",
        Some("token-a"),
        None,
    )
    .expect("proof creation should succeed");

    let result = verify_dpop_proof(
        &proof,
        "GET",
        "https://api.example.com/resource",
        Some("token-b"),
        300,
    );
    assert!(result.is_err());
}

#[test]
fn dpop_proof_rejects_wrong_htm() {
    let pair = generate_key_pair();
    let proof = create_dpop_proof(&pair, "GET", "https://api.example.com/resource", None, None)
        .expect("proof creation should succeed");

    let result = verify_dpop_proof(
        &proof,
        "POST",
        "https://api.example.com/resource",
        None,
        300,
    );
    assert!(result.is_err());
}

#[test]
fn dpop_proof_rejects_wrong_htu() {
    let pair = generate_key_pair();
    let proof = create_dpop_proof(&pair, "GET", "https://api.example.com/resource", None, None)
        .expect("proof creation should succeed");

    let result = verify_dpop_proof(&proof, "GET", "https://api.example.com/other", None, 300);
    assert!(result.is_err());
}
