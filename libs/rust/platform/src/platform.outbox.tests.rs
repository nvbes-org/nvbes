use super::*;
use crate::event::{DomainEventEnvelope, DomainEventInput};
use serde_json::json;

fn event() -> DomainEventEnvelope {
    DomainEventEnvelope::new(DomainEventInput {
        event_type: "identity.user.created".to_string(),
        event_version: 1,
        tenant_id: Uuid::new_v4(),
        region_id: "eu-fr".to_string(),
        correlation_id: Uuid::new_v4(),
        idempotency_key: "signup:object".to_string(),
        payload: json!({ "object_id": Uuid::new_v4() }),
    })
    .expect("event must be valid")
}

#[test]
fn creates_deduplicated_outbox_message() {
    let message = OutboxMessage::for_aggregate(Uuid::new_v4(), event())
        .expect("outbox message must be valid");

    assert!(message.partition_key.ends_with(":eu-fr"));
    assert_eq!(message.dedupe_key, "identity.user.created:1:signup:object");
    assert!(message.payload().is_object());
}

#[test]
fn rejects_missing_aggregate() {
    assert_eq!(
        OutboxMessage::for_aggregate(Uuid::nil(), event()).expect_err("nil aggregate id must fail"),
        OutboxMessageError::MissingAggregate
    );
}
