use proptest::prelude::*;

use super::{compute_ath, verify_dpop_proof};

proptest! {
    #[test]
    fn arbitrary_dpop_input_never_panics(
        proof in proptest::collection::vec(any::<u8>(), 0..4096),
        method in "[A-Z]{1,16}",
        url in "https://[a-z0-9.]{1,80}/[a-z0-9/_-]{0,120}",
    ) {
        let proof = String::from_utf8_lossy(&proof);
        let _ = verify_dpop_proof(&proof, &method, &url, None, 300);
    }

    #[test]
    fn access_token_hash_is_stable_and_base64url(token in ".{0,2048}") {
        let first = compute_ath(&token);
        let second = compute_ath(&token);

        prop_assert_eq!(&first, &second);
        prop_assert_eq!(first.len(), 43);
        let is_base64url = first.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
        });
        prop_assert!(is_base64url);
    }
}
