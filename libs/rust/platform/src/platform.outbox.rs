use crate::event::DomainEventEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutboxMessage {
    pub message_id: Uuid,
    pub aggregate_id: Uuid,
    pub event: DomainEventEnvelope,
    pub partition_key: String,
    pub dedupe_key: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OutboxMessageError {
    #[error("aggregate_id is required")]
    MissingAggregate,
    #[error("partition_key is required")]
    MissingPartitionKey,
}

impl OutboxMessage {
    pub fn for_aggregate(
        aggregate_id: Uuid,
        event: DomainEventEnvelope,
    ) -> Result<Self, OutboxMessageError> {
        if aggregate_id.is_nil() {
            return Err(OutboxMessageError::MissingAggregate);
        }

        let partition_key = format!("{}:{}", event.tenant_id, event.region_id);
        if partition_key.trim().is_empty() {
            return Err(OutboxMessageError::MissingPartitionKey);
        }

        Ok(Self {
            message_id: Uuid::new_v4(),
            aggregate_id,
            dedupe_key: format!(
                "{}:{}:{}",
                event.event_type, event.event_version, event.idempotency_key
            ),
            event,
            partition_key,
        })
    }

    pub fn payload(&self) -> &Value {
        &self.event.payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{DomainEventEnvelope, DomainEventInput};
    use serde_json::json;

    fn event() -> DomainEventEnvelope {
        DomainEventEnvelope::new(DomainEventInput {
            event_type: "drive.file.created".to_string(),
            event_version: 1,
            tenant_id: Uuid::new_v4(),
            region_id: "eu-fr".to_string(),
            correlation_id: Uuid::new_v4(),
            idempotency_key: "upload:object".to_string(),
            payload: json!({ "object_id": Uuid::new_v4() }),
        })
        .expect("event must be valid")
    }

    #[test]
    fn creates_deduplicated_outbox_message() {
        let message = OutboxMessage::for_aggregate(Uuid::new_v4(), event())
            .expect("outbox message must be valid");

        assert!(message.partition_key.ends_with(":eu-fr"));
        assert_eq!(message.dedupe_key, "drive.file.created:1:upload:object");
        assert!(message.payload().is_object());
    }

    #[test]
    fn rejects_missing_aggregate() {
        assert_eq!(
            OutboxMessage::for_aggregate(Uuid::nil(), event())
                .expect_err("nil aggregate id must fail"),
            OutboxMessageError::MissingAggregate
        );
    }
}
