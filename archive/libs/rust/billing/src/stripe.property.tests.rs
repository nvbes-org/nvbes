use chrono::Utc;
use proptest::prelude::*;
use serde_json::json;

use super::{
    STRIPE_WEBHOOK_TOLERANCE_SECONDS, constant_time_eq, hmac_sha256_hex, parse_stripe_event,
    parse_stripe_signature_header, stripe_subscription_status, verify_stripe_signature,
};

proptest! {
    #[test]
    fn parse_signature_header_never_panics_on_arbitrary_input(header in ".*") {
        let _ = parse_stripe_signature_header(&header);
    }

    #[test]
    fn parse_signature_header_extracts_timestamp_and_signatures(
        timestamp in any::<i64>().prop_map(|t| t.abs().to_string()),
        signatures in prop::collection::vec("[a-f0-9]{64}", 1..4)
    ) {
        let sig_parts = signatures.iter().map(|s| format!("v1={s}")).collect::<Vec<_>>().join(",");
        let header = format!("t={timestamp},{sig_parts}");

        let parsed = parse_stripe_signature_header(&header);
        prop_assert!(parsed.is_some());
        let (parsed_ts, parsed_sigs) = parsed.unwrap();
        prop_assert_eq!(parsed_ts, timestamp);
        prop_assert_eq!(parsed_sigs, signatures);
    }

    #[test]
    fn parse_signature_header_rejects_missing_required_fields(
        field in "[a-zA-Z0-9_]{1,16}",
        val in "[a-zA-Z0-9_]{1,16}"
    ) {
        prop_assume!(field != "t" && field != "v1");
        let header = format!("{field}={val}");
        prop_assert!(parse_stripe_signature_header(&header).is_none());
    }

    #[test]
    fn verify_stripe_signature_never_panics_on_arbitrary_input(
        secret in ".*",
        header in ".*",
        payload in prop::collection::vec(any::<u8>(), 0..256)
    ) {
        let _ = verify_stripe_signature(&secret, Some(&header), &payload);
    }

    #[test]
    fn verify_stripe_signature_accepts_valid_fresh_signature(
        secret in "[a-zA-Z0-9_]{16,32}",
        event_id in "[a-z0-9_]{10,24}",
        offset_seconds in 0i64..120
    ) {
        let payload = format!(r#"{{"id":"{event_id}","type":"checkout.session.completed"}}"#).into_bytes();
        let timestamp = Utc::now().timestamp() - offset_seconds;
        let signed_payload = format!("{timestamp}.{}", String::from_utf8_lossy(&payload));
        let signature = hmac_sha256_hex(secret.as_bytes(), signed_payload.as_bytes());
        let header = format!("t={timestamp},v1={signature}");

        prop_assert!(verify_stripe_signature(&secret, Some(&header), &payload).is_ok());

        // Tampered payload fails
        let mut tampered = payload.clone();
        tampered.push(b' ');
        prop_assert!(verify_stripe_signature(&secret, Some(&header), &tampered).is_err());

        // Tampered secret fails
        let wrong_secret = format!("{secret}_wrong");
        prop_assert!(verify_stripe_signature(&wrong_secret, Some(&header), &payload).is_err());
    }

    #[test]
    fn verify_stripe_signature_rejects_expired_signature(
        secret in "[a-zA-Z0-9_]{16,32}",
        excess_seconds in 1i64..1000
    ) {
        let payload = b"{\"id\":\"evt_expired\"}";
        let timestamp = Utc::now().timestamp() - STRIPE_WEBHOOK_TOLERANCE_SECONDS - excess_seconds;
        let signed_payload = format!("{timestamp}.{}", String::from_utf8_lossy(payload));
        let signature = hmac_sha256_hex(secret.as_bytes(), signed_payload.as_bytes());
        let header = format!("t={timestamp},v1={signature}");

        prop_assert_eq!(
            verify_stripe_signature(&secret, Some(&header), payload),
            Err("stale_stripe_signature")
        );
    }

    #[test]
    fn parse_stripe_event_never_panics_on_arbitrary_bytes(
        bytes in prop::collection::vec(any::<u8>(), 0..256)
    ) {
        let _ = parse_stripe_event(&bytes);
    }

    #[test]
    fn parse_stripe_event_roundtrips_valid_event(
        id in "evt_[a-zA-Z0-9]{16,24}",
        event_type in "[a-z_]+\\.[a-z_]+",
        livemode in any::<bool>()
    ) {
        let payload = json!({
            "id": id,
            "type": event_type,
            "livemode": livemode,
            "data": {
                "object": {
                    "id": "sub_test_123"
                }
            }
        });
        let bytes = serde_json::to_vec(&payload).unwrap();
        let parsed = parse_stripe_event(&bytes);
        prop_assert!(parsed.is_some());
        let event = parsed.unwrap();
        prop_assert_eq!(event.id, id);
        prop_assert_eq!(event.event_type, event_type);
        prop_assert_eq!(event.livemode, livemode);
        prop_assert_eq!(&event.data_object["id"], "sub_test_123");
    }

    #[test]
    fn constant_time_eq_matches_standard_equality(
        a in prop::collection::vec(any::<u8>(), 0..64),
        b in prop::collection::vec(any::<u8>(), 0..64)
    ) {
        prop_assert_eq!(constant_time_eq(&a, &b), a == b);
        prop_assert!(constant_time_eq(&a, &a)); // reflexivity
        prop_assert_eq!(constant_time_eq(&a, &b), constant_time_eq(&b, &a)); // symmetry
    }

    #[test]
    fn stripe_subscription_status_maps_canonical_or_suspends(
        status in ".*"
    ) {
        let canonical = stripe_subscription_status(Some(&status));
        prop_assert!(matches!(
            canonical,
            "active" | "trialing" | "past_due" | "canceled" | "incomplete" | "suspended"
        ));
    }
}
