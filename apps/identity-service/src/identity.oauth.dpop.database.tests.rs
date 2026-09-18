use super::verify_and_consume_code_proof;
use crate::test_fixtures::database;
use nvbes_dpop::{generate_key_pair, proof::create_dpop_proof};

#[tokio::test]
async fn dpop_jti_is_consumed_once_per_key() {
    let db = database().await;
    let pair = generate_key_pair();
    let proof = create_dpop_proof(
        &pair,
        "POST",
        "https://identity.example/oauth/token",
        None,
        None,
    )
    .unwrap();
    let first =
        verify_and_consume_code_proof(&db, &proof, "POST", "https://identity.example/oauth/token")
            .await
            .unwrap();
    assert_eq!(first, pair.jkt);
    assert!(
        verify_and_consume_code_proof(&db, &proof, "POST", "https://identity.example/oauth/token")
            .await
            .is_err()
    );
}
