#[path = "platform.event.rs"]
pub mod event;
#[path = "platform.outbox.rs"]
pub mod outbox;

pub use event::{DomainEventEnvelope, DomainEventError, DomainEventInput};
pub use outbox::{OutboxMessage, OutboxMessageError};
