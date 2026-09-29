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

#[test]
fn valid_signatures_do_not_allow_private_jwks_or_wrong_curve_metadata() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
    use p256::ecdsa::signature::Signer;
    let pair = generate_key_pair();
    let proof = create_dpop_proof(
        &pair,
        "GET",
        "https://api.example/resource",
        Some("token"),
        None,
    )
    .unwrap();
    let parts: Vec<_> = proof.split('.').collect();
    let header: serde_json::Value = serde_json::from_slice(&B64.decode(parts[0]).unwrap()).unwrap();
    for field in ["d", "crv", "crit", "x"] {
        let mut changed = header.clone();
        match field {
            "d" => changed["jwk"]["d"] = serde_json::json!("private-material-must-not-be-sent"),
            "crv" => changed["jwk"]["crv"] = serde_json::json!("P-384"),
            "crit" => changed["crit"] = serde_json::json!(["unsupported"]),
            _ => changed["jwk"]["x"] = serde_json::json!(format!("{}=", pair.jwk.x)),
        }
        let input = format!(
            "{}.{}",
            B64.encode(serde_json::to_vec(&changed).unwrap()),
            parts[1]
        );
        let signature: p256::ecdsa::Signature = pair.signing_key.sign(input.as_bytes());
        let altered = format!("{input}.{}", B64.encode(signature.to_bytes()));
        assert!(
            verify_dpop_proof(
                &altered,
                "GET",
                "https://api.example/resource",
                Some("token"),
                300
            )
            .is_err(),
            "accepted {field}"
        );
    }
}
