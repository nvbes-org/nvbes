pub use crate::models::{BillingStateRecord, PlanRecord, ProviderPriceMapping};
pub use sqlx::{Postgres, Row, Transaction};
pub use uuid::Uuid;

#[path = "db.fraud.rs"]
pub mod fraud;
#[path = "db.plans_and_state.rs"]
mod plans_and_state;
#[path = "db.price_mappings.rs"]
mod price_mappings;
#[path = "db.provider_customers.rs"]
mod provider_customers;
#[path = "db.provider_events.rs"]
mod provider_events;
#[path = "db.provider_invoices.rs"]
mod provider_invoices;
#[path = "db.provider_payment_methods.rs"]
mod provider_payment_methods;
#[path = "db.provider_routing.rs"]
mod provider_routing;
#[path = "db.provider_subscription_context.rs"]
mod provider_subscription_context;
#[path = "db.provider_subscriptions.rs"]
mod provider_subscriptions;
#[path = "db.workspace_projection.rs"]
mod workspace_projection;

pub use fraud::{
    BillingFraudAssessmentInput, BillingFraudVelocity, billing_fraud_velocity_tx,
    insert_billing_fraud_assessment_tx, insert_billing_fraud_psp_signal_tx,
};
pub use plans_and_state::fetch_billing_state_tx;
pub use price_mappings::{
    ensure_plan_seeded_tx, fetch_active_price_mapping_tx, fetch_plan_by_code_tx,
    plan_id_for_provider_price_tx, workspace_id_for_provider_customer_tx,
};
pub use provider_customers::{upsert_provider_customer_mapping_tx, upsert_provider_customer_tx};
pub use provider_events::{
    mark_provider_event_failed, mark_provider_event_processed, mark_provider_event_replayed,
    record_provider_event,
};
pub use provider_invoices::{
    ProviderInvoiceInput, ProviderInvoiceLineInput, upsert_provider_invoice_tx,
};
pub use provider_payment_methods::upsert_provider_payment_method_tx;
pub use provider_routing::{
    ProviderRoutingRule, ProviderRoutingRuleLookupError, fetch_provider_routing_rule_tx,
};
pub use provider_subscription_context::{
    ProviderSubscriptionContext, provider_subscription_context_tx,
};
pub use provider_subscriptions::{
    activate_mollie_subscription_after_initial_payment_tx, upsert_provider_subscription_tx,
    workspace_id_for_provider_customer_code_tx, workspace_id_for_provider_subscription_tx,
};
pub use workspace_projection::{
    BillingAuditEventInput, insert_billing_audit_event_tx, insert_billing_audit_tx,
    plan_id_by_code_tx, project_workspace_plan_tx, tenant_id_for_workspace_tx,
};
