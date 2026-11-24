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
#[path = "platform.outbox.tests.rs"]
mod tests;
