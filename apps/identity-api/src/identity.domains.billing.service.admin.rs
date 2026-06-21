#[path = "identity.domains.billing.service.admin.mutations.rs"]
mod mutations;
#[path = "identity.domains.billing.service.admin.operations.rs"]
mod operations;
#[path = "identity.domains.billing.service.admin.provider_ops.rs"]
mod provider_ops;
#[path = "identity.domains.billing.service.admin.search.rs"]
mod search;

pub use mutations::{
    BillingAdminMutationInput, BillingAdminMutationResult, BillingRefundIntentInput,
    create_credit_note, create_refund_intent, create_write_off,
};
pub use operations::{
    BillingGraceOverrideInput, BillingManualCompInput, create_manual_compensation,
    override_grace_period,
};
pub use provider_ops::{
    BillingProviderEventReplayInput, BillingProviderEventReplayResult,
    BillingProviderMigrationInput, create_provider_migration_run, replay_provider_event,
};
pub use search::{BillingAdminSearchResult, search_billing_admin};

pub fn audit_reason_is_valid(reason: &str) -> bool {
    reason.trim().len() >= 12
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingAdminAction {
    Refund,
    CreditNote,
    WriteOff,
    OverrideGrace,
    ManualComp,
    ReplayProviderEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingRunbook {
    PspOutage,
    WebhookLag,
    DuplicatePayment,
    TaxConfigError,
    LedgerImbalance,
    FailedExport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingAdminDecision {
    pub allowed: bool,
    pub requires_step_up_mfa: bool,
    pub reason_required: bool,
}

pub fn authorize_billing_admin_action(has_admin_scope: bool, reason: &str) -> BillingAdminDecision {
    BillingAdminDecision {
        allowed: has_admin_scope && audit_reason_is_valid(reason),
        requires_step_up_mfa: true,
        reason_required: true,
    }
}

pub fn runbook_slug(runbook: BillingRunbook) -> &'static str {
    match runbook {
        BillingRunbook::PspOutage => "psp-outage",
        BillingRunbook::WebhookLag => "webhook-lag",
        BillingRunbook::DuplicatePayment => "duplicate-payment",
        BillingRunbook::TaxConfigError => "tax-config-error",
        BillingRunbook::LedgerImbalance => "ledger-imbalance",
        BillingRunbook::FailedExport => "failed-export",
    }
}

pub fn replay_is_idempotent(
    provider: &str,
    provider_event_id: &str,
    previous_replay_count: u32,
) -> String {
    format!("{provider}:{provider_event_id}:replay-{previous_replay_count}")
}

pub(crate) fn validate_admin_mutation(
    amount_minor: i64,
    reason: &str,
) -> Result<(), crate::http::error::AppError> {
    if amount_minor <= 0 {
        return Err(crate::http::error::AppError::bad_request(
            "invalid_amount",
            "Billing admin amount must be greater than zero.",
        ));
    }
    if !audit_reason_is_valid(reason) {
        return Err(crate::http::error::AppError::bad_request(
            "audit_reason_required",
            "Billing admin actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

pub(crate) fn validate_provider_code(provider: &str) -> Result<(), crate::http::error::AppError> {
    if matches!(provider, "stripe" | "mollie") {
        return Ok(());
    }
    Err(crate::http::error::AppError::bad_request(
        "invalid_billing_provider",
        "Billing provider must be stripe or mollie.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_admin_action_requires_scope_step_up_and_reason() {
        let denied = authorize_billing_admin_action(false, "customer requested refund");
        assert!(!denied.allowed);
        assert!(denied.requires_step_up_mfa);
        assert!(denied.reason_required);

        let allowed = authorize_billing_admin_action(true, "customer requested refund");
        assert!(allowed.allowed);
    }

    #[test]
    fn billing_admin_runbook_slugs_cover_critical_incidents() {
        assert_eq!(runbook_slug(BillingRunbook::PspOutage), "psp-outage");
        assert_eq!(
            runbook_slug(BillingRunbook::LedgerImbalance),
            "ledger-imbalance"
        );
    }

    #[test]
    fn billing_admin_replay_key_is_deterministic() {
        assert_eq!(
            replay_is_idempotent("stripe", "evt_1", 0),
            "stripe:evt_1:replay-0"
        );
    }

    #[test]
    fn billing_admin_mutation_rejects_missing_reason_and_non_positive_amount() {
        assert!(validate_admin_mutation(100, "customer approved credit").is_ok());
        assert!(validate_admin_mutation(0, "customer approved credit").is_err());
        assert!(validate_admin_mutation(100, "short").is_err());
    }

    #[test]
    fn provider_code_validation_accepts_supported_psps_only() {
        assert!(validate_provider_code("stripe").is_ok());
        assert!(validate_provider_code("mollie").is_ok());
        assert!(validate_provider_code("paypal").is_err());
    }
}
