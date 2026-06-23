use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BackofficeAccess {
    pub(crate) tenant_id: Uuid,
    pub(crate) actor_principal_id: Uuid,
}

#[derive(Debug, Serialize)]
pub(crate) struct MutationResult {
    pub(crate) object_id: Uuid,
    pub(crate) ledger_entry_count: u64,
    pub(crate) audit_action: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProviderReplayResult {
    pub(crate) object_id: Uuid,
    pub(crate) provider: String,
    pub(crate) provider_event_id: String,
    pub(crate) status: String,
    pub(crate) audit_action: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchResult {
    pub(crate) kind: String,
    pub(crate) id: Uuid,
    pub(crate) label: String,
    pub(crate) status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SearchQuery {
    pub(crate) q: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreditNoteRequest {
    pub(crate) invoice_id: Uuid,
    pub(crate) amount_minor: i64,
    pub(crate) currency: String,
    pub(crate) reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RefundIntentRequest {
    pub(crate) payment_id: Uuid,
    pub(crate) provider: String,
    pub(crate) amount_minor: i64,
    pub(crate) currency: String,
    pub(crate) reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderReplayRequest {
    pub(crate) provider: String,
    pub(crate) provider_event_id: String,
    pub(crate) reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderMigrationRequest {
    pub(crate) from_provider: String,
    pub(crate) to_provider: String,
    pub(crate) reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GraceOverrideRequest {
    pub(crate) subscription_id: Option<Uuid>,
    pub(crate) grace_days: i64,
    pub(crate) reason: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ManualCompRequest {
    pub(crate) amount_minor: i64,
    pub(crate) currency: String,
    pub(crate) direction: String,
    pub(crate) reason: String,
}
