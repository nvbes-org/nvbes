#[path = "platform.event.rs"]
pub mod event;
#[path = "platform.finops.budget.rs"]
pub mod finops_budget;
#[path = "platform.outbox.rs"]
pub mod outbox;

#[path = "platform.cockpit.model.rs"]
pub mod cockpit_model;
#[path = "platform.cockpit.health.rs"]
pub mod cockpit_health;
#[path = "platform.cockpit.jobs.rs"]
pub mod cockpit_jobs;
#[path = "platform.cockpit.trust_risk.rs"]
pub mod cockpit_trust_risk;
#[path = "platform.cockpit.email.rs"]
pub mod cockpit_email;
#[path = "platform.cockpit.billing.rs"]
pub mod cockpit_billing;
#[path = "platform.cockpit.finops.rs"]
pub mod cockpit_finops;
#[path = "platform.cockpit.backup.rs"]
pub mod cockpit_backup;
#[path = "platform.cockpit.degraded.rs"]
pub mod cockpit_degraded;
#[path = "platform.cockpit.actions.rs"]
pub mod cockpit_actions;
#[path = "platform.cockpit.auth.rs"]
pub mod cockpit_auth;
#[path = "platform.cockpit.ui.rs"]
pub mod cockpit_ui;
#[path = "platform.cockpit.server.rs"]
pub mod cockpit_server;

pub use event::{DomainEventEnvelope, DomainEventError, DomainEventInput};
pub use finops_budget::{BudgetPolicyError, BudgetStage, BudgetThresholds};
pub use outbox::{OutboxMessage, OutboxMessageError};
pub use cockpit_model::{
    AggregateHealth, CockpitOverview, HealthStatus, RuntimeHealth, ServiceId,
};
pub use cockpit_server::{create_platform_cockpit_router, PlatformCockpitState};
