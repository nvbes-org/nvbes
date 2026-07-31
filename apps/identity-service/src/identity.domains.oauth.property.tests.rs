use proptest::prelude::*;

use super::{
    client_assertion::client_id_from_unverified_assertion, rar::parse_authorization_details,
};

proptest! {
    #[test]
    fn arbitrary_authorization_details_never_panic(
        raw in proptest::collection::vec(any::<u8>(), 0..8192),
    ) {
        let raw = String::from_utf8_lossy(&raw);
        if let Ok(details) = parse_authorization_details(Some(&raw)) {
            prop_assert!(details.len() <= 16);
            for detail in details {
                let detail_type = detail
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                prop_assert!(!detail_type.trim().is_empty());
            }
        }
    }

    #[test]
    fn arbitrary_client_assertion_never_panics(
        assertion in proptest::collection::vec(any::<u8>(), 0..8192),
        client_id in proptest::option::of("[a-zA-Z0-9._-]{0,128}"),
    ) {
        let assertion = String::from_utf8_lossy(&assertion);
        let _ = client_id_from_unverified_assertion(client_id.as_deref(), &assertion);
    }
}
