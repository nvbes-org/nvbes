#[path = "platform.event.rs"]
pub mod event;
#[path = "platform.finops.budget.rs"]
pub mod finops_budget;
#[path = "platform.outbox.rs"]
pub mod outbox;

pub use event::{DomainEventEnvelope, DomainEventError, DomainEventInput};
pub use finops_budget::{BudgetPolicyError, BudgetStage, BudgetThresholds};
pub use outbox::{OutboxMessage, OutboxMessageError};
