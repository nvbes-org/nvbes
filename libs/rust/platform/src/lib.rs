#[path = "platform.event.rs"]
pub mod event;
#[path = "platform.finops.budget.rs"]
pub mod finops_budget;
#[path = "platform.outbox.rs"]
pub mod outbox;

#[path = "platform.cockpit.actions.rs"]
pub mod cockpit_actions;
#[path = "platform.cockpit.auth.rs"]
pub mod cockpit_auth;
#[path = "platform.cockpit.backup.rs"]
pub mod cockpit_backup;
#[path = "platform.cockpit.billing.rs"]
pub mod cockpit_billing;
#[path = "platform.cockpit.degraded.rs"]
pub mod cockpit_degraded;
#[path = "platform.cockpit.email.rs"]
pub mod cockpit_email;
#[path = "platform.cockpit.finops.rs"]
pub mod cockpit_finops;
#[path = "platform.cockpit.health.rs"]
pub mod cockpit_health;
#[path = "platform.cockpit.jobs.rs"]
pub mod cockpit_jobs;
#[path = "platform.cockpit.model.rs"]
pub mod cockpit_model;
#[path = "platform.cockpit.server.rs"]
pub mod cockpit_server;
#[path = "platform.cockpit.trust_risk.rs"]
pub mod cockpit_trust_risk;

#[path = "platform.operations.context.rs"]
pub mod operations_context;
#[path = "platform.operations.db.rs"]
pub mod operations_db;
#[path = "platform.operations.error.rs"]
pub mod operations_error;
#[path = "platform.operations.model.rs"]
pub mod operations_model;
#[path = "platform.operations.routes.rs"]
pub mod operations_routes;
#[path = "platform.operations.service.rs"]
pub mod operations_service;
#[cfg(all(test, feature = "database-tests"))]
#[path = "platform.operations.tests.rs"]
mod operations_tests;
#[path = "platform.operations.validation.rs"]
pub mod operations_validation;

pub use cockpit_model::{AggregateHealth, CockpitOverview, HealthStatus, RuntimeHealth, ServiceId};
pub use cockpit_server::{PlatformCockpitState, create_platform_cockpit_router};
pub use event::{DomainEventEnvelope, DomainEventError, DomainEventInput};
pub use finops_budget::{BudgetPolicyError, BudgetStage, BudgetThresholds};
pub use outbox::{OutboxMessage, OutboxMessageError};
