use crate::proto::nvbes::email::v1::{
    EmailPrivacyActivity, EmailPrivacyMessage, EmailPrivacyProviderEvent,
};

use super::{EmailOperationsError, privacy_activity_json};

#[test]
fn privacy_json_rejects_missing_or_invalid_timestamps() {
    let valid = Some(prost_types::Timestamp {
        seconds: 1_700_000_000,
        nanos: 123,
    });
    let message = |accepted_at| EmailPrivacyMessage {
        id: "message".into(),
        producer: "producer".into(),
        business_type: "type".into(),
        template_version: 1,
        status: "sent".into(),
        accepted_at,
        provider_accepted_at: valid,
        terminal_at: None,
        deliver_before: valid,
        updated_at: valid,
    };
    for invalid in [
        None,
        Some(prost_types::Timestamp {
            seconds: 0,
            nanos: -1,
        }),
        Some(prost_types::Timestamp {
            seconds: i64::MAX,
            nanos: 0,
        }),
    ] {
        let activity = EmailPrivacyActivity {
            messages: vec![message(invalid)],
            provider_events: Vec::new(),
        };
        assert!(matches!(
            privacy_activity_json(activity),
            Err(EmailOperationsError::Protocol)
        ));
    }
    let activity = EmailPrivacyActivity {
        messages: vec![message(valid)],
        provider_events: vec![EmailPrivacyProviderEvent {
            id: "event".into(),
            provider: "provider".into(),
            event_type: "delivered".into(),
            received_at: valid,
            processed_at: None,
            processing_result: None,
        }],
    };
    let json = privacy_activity_json(activity).expect("valid privacy JSON");
    assert!(
        json["messages"][0]["provider_accepted_at"]
            .as_str()
            .unwrap()
            .contains("2023")
    );
    assert!(json["provider_events"][0]["processed_at"].is_null());
}
