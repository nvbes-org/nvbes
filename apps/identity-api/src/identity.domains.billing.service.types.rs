pub use billing_shared::models::{BillingStateRecord, PlanRecord, StripePriceMapping};
pub use billing_shared::types::*;
use nvbes_billing as billing_shared;
use serde::Serialize;
use uuid::Uuid;

// AuditEventInput is still local for now as it uses AppState/AppError context in some places,
// or it's very specific to identity-api's audit log format.
#[derive(Debug, Serialize)]
pub(crate) struct AuditEventInput<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: serde_json::Value,
}
