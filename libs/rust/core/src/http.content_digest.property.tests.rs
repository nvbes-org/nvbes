use proptest::prelude::*;

use super::{content_digest_header_value, parse_sha256_digest, sha256_digest_base64};

proptest! {
    #[test]
    fn arbitrary_input_never_panics(raw in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let input = String::from_utf8_lossy(&raw);
        let _ = parse_sha256_digest(&input);
    }

    #[test]
    fn round_trip_content_digest_header(body in proptest::collection::vec(any::<u8>(), 0..8192)) {
        let expected_hash = sha256_digest_base64(&body);
        let header_value = content_digest_header_value(&body);

        let parsed = parse_sha256_digest(&header_value);
        prop_assert_eq!(parsed, Some(expected_hash));
    }

    #[test]
    fn sha256_digest_base64_length_and_charset(body in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let hash = sha256_digest_base64(&body);
        prop_assert_eq!(hash.len(), 44);
        prop_assert!(hash.ends_with('='));
        let is_valid_base64 = hash.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='
        });
        prop_assert!(is_valid_base64);
    }
}
