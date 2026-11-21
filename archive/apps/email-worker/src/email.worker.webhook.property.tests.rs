use axum::http::{HeaderMap, HeaderValue, header};
use proptest::prelude::*;

use super::{SnsMessage, is_sns_content_type, webhook_event_type};

proptest! {
    #[test]
    fn arbitrary_bytes_sns_deserialize_never_panics(raw in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = serde_json::from_slice::<SnsMessage>(&raw);
    }

    #[test]
    fn webhook_event_type_determinism_and_invariants(
        message_type in "\\PC{0,50}",
        inner_message in "\\PC{0,1024}",
    ) {
        let message = SnsMessage {
            message_type: message_type.clone(),
            message_id: "test-id".to_string(),
            topic_arn: "arn:aws:sns:eu-west-3:123456789012:topic".to_string(),
            message: inner_message.clone(),
            timestamp: "2026-08-05T00:00:00Z".to_string(),
            signature_version: "1".to_string(),
            signature: "sig".to_string(),
            signing_cert_url: "https://cert.test".to_string(),
            subject: None,
            token: None,
            subscribe_url: None,
        };

        let res1 = webhook_event_type(&message);
        let res2 = webhook_event_type(&message);
        prop_assert_eq!(&res1, &res2);

        if message_type == "SubscriptionConfirmation" {
            prop_assert_eq!(res1, "subscription_confirmation");
        }
    }

    #[test]
    fn is_sns_content_type_robustness(
        prefix in "[a-zA-Z0-9/_-]{0,30}",
        params in proptest::option::of("; [a-zA-Z0-9=_-]{1,20}"),
    ) {
        let header_str = format!("{}{}", prefix, params.unwrap_or_default());
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&header_str) {
            headers.insert(header::CONTENT_TYPE, val);
            let is_sns = is_sns_content_type(&headers);

            let trimmed_mime = prefix.trim().to_ascii_lowercase();
            let expected = matches!(trimmed_mime.as_str(), "application/json" | "text/plain");
            prop_assert_eq!(is_sns, expected);
        }
    }

    #[test]
    fn email_address_parser_never_panics(raw in "\\PC{0,256}") {
        let parsed = raw.trim().parse::<lettre::Address>();
        if !raw.contains('@') || raw.contains('\0') || raw.contains('\n') || raw.contains('\r') {
            prop_assert!(parsed.is_err());
        }
    }

    #[test]
    fn valid_email_address_always_accepted(
        local in "[a-zA-Z0-9._%+-]{1,20}",
        domain in "[a-zA-Z0-9.-]{2,20}",
        tld in "[a-zA-Z]{2,6}",
    ) {
        let email = format!("{local}@{domain}.{tld}");
        if let Ok(addr) = email.parse::<lettre::Address>() {
            prop_assert_eq!(addr.to_string(), email);
        }
    }
}
