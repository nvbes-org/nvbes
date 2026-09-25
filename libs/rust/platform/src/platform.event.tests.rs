use super::*;
use serde_json::json;

fn valid_input() -> DomainEventInput {
    DomainEventInput {
        event_type: "identity.user.created".to_string(),
        event_version: 1,
        tenant_id: Uuid::new_v4(),
        region_id: "eu-fr".to_string(),
        correlation_id: Uuid::new_v4(),
        idempotency_key: "signup:tenant:user".to_string(),
        payload: json!({ "user_id": Uuid::new_v4() }),
    }
}

#[test]
fn builds_versioned_domain_event_envelope() {
    let event = DomainEventEnvelope::new(valid_input()).expect("event input must be valid");

    assert_eq!(event.event_type, "identity.user.created");
    assert_eq!(event.event_version, 1);
    assert_eq!(event.region_id, "eu-fr");
    assert!(event.payload.is_object());
}

#[test]
fn rejects_zero_event_version() {
    let mut input = valid_input();
    input.event_version = 0;
    assert_eq!(
        DomainEventEnvelope::new(input).expect_err("version zero must fail"),
        DomainEventError::InvalidVersion
    );
}

#[test]
fn rejects_uppercase_and_hyphen_event_type_segments() {
    let mut input = valid_input();
    input.event_type = "Identity.user.created".to_string();
    assert_eq!(
        DomainEventEnvelope::new(input).expect_err("uppercase must fail"),
        DomainEventError::InvalidEventType
    );

    let mut input = valid_input();
    input.event_type = "identity.user-created.v1".to_string();
    assert_eq!(
        DomainEventEnvelope::new(input).expect_err("hyphen must fail"),
        DomainEventError::InvalidEventType
    );

    let mut input = valid_input();
    input.event_type = "identity.user_created.v1".to_string();
    DomainEventEnvelope::new(input).expect("underscore segments must be accepted");
}

#[test]
fn rejects_missing_region_idempotency_payload_and_short_event_type() {
    let mut input = valid_input();
    input.region_id = "  ".to_string();
    assert_eq!(
        DomainEventEnvelope::new(input).expect_err("blank region must fail"),
        DomainEventError::MissingRegion
    );

    let mut input = valid_input();
    input.idempotency_key = "".to_string();
    assert_eq!(
        DomainEventEnvelope::new(input).expect_err("blank idempotency must fail"),
        DomainEventError::MissingIdempotencyKey
    );

    let mut input = valid_input();
    input.payload = json!(["not", "an", "object"]);
    assert_eq!(
        DomainEventEnvelope::new(input).expect_err("array payload must fail"),
        DomainEventError::InvalidPayload
    );

    let mut input = valid_input();
    input.event_type = "identity.user".to_string();
    assert_eq!(
        DomainEventEnvelope::new(input).expect_err("two-part event type must fail"),
        DomainEventError::InvalidEventType
    );

    let mut input = valid_input();
    input.event_type = "identity..user".to_string();
    assert_eq!(
        DomainEventEnvelope::new(input).expect_err("empty segment must fail"),
        DomainEventError::InvalidEventType
    );
}
