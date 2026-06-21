use crate::error::PortError;
use nvbes_platform::OutboxMessage;

pub trait OutboxPort {
    fn enqueue(&mut self, message: OutboxMessage) -> Result<(), PortError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvbes_platform::{DomainEventEnvelope, DomainEventInput};
    use serde_json::json;
    use uuid::Uuid;

    #[derive(Default)]
    struct MemoryOutbox {
        messages: Vec<OutboxMessage>,
    }

    impl OutboxPort for MemoryOutbox {
        fn enqueue(&mut self, message: OutboxMessage) -> Result<(), PortError> {
            self.messages.push(message);
            Ok(())
        }
    }

    #[test]
    fn outbox_port_accepts_platform_message() {
        let event = DomainEventEnvelope::new(DomainEventInput {
            event_type: "workspace.membership.created".to_string(),
            event_version: 1,
            tenant_id: Uuid::new_v4(),
            region_id: "eu-fr".to_string(),
            correlation_id: Uuid::new_v4(),
            idempotency_key: "membership:create".to_string(),
            payload: json!({ "membership_id": Uuid::new_v4() }),
        })
        .expect("event must be valid");
        let message =
            OutboxMessage::for_aggregate(Uuid::new_v4(), event).expect("message must be valid");

        let mut outbox = MemoryOutbox::default();
        outbox.enqueue(message).expect("enqueue must pass");

        assert_eq!(outbox.messages.len(), 1);
    }
}
