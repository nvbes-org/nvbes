use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::cockpit_model::BillingReconciliationSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReconciliationMismatch {
    pub mismatch_id: Uuid,
    pub customer_id: String,
    pub provider_event_id: String,
    pub provider_status: String,
    pub local_status: String,
    pub difference_summary: String,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BillingCockpitError {
    #[error("live stripe keys/webhooks are strictly forbidden in V1")]
    LiveModeForbidden,
    #[error("automated subscription changes are forbidden: operator manual decision required")]
    AutomatedChangeForbidden,
    #[error("reconciliation reason is required (minimum 3 chars)")]
    InvalidReconciliationReason,
}

pub struct BillingCockpitView;

impl BillingCockpitView {
    pub fn build_summary(
        webhook_events_24h: u64,
        unverified_webhooks_count: u64,
        pending_reconciliations_count: u64,
        reconciliation_mismatch_count: u64,
        last_reconciliation_at: Option<DateTime<Utc>>,
    ) -> BillingReconciliationSummary {
        BillingReconciliationSummary {
            test_mode: true,
            webhook_events_24h,
            unverified_webhooks_count,
            pending_reconciliations_count,
            reconciliation_mismatch_count,
            last_reconciliation_at,
        }
    }

    pub fn assert_test_mode(key: &str) -> Result<(), BillingCockpitError> {
        if key.starts_with("sk_live_") || key.starts_with("whsec_live_") {
            return Err(BillingCockpitError::LiveModeForbidden);
        }
        Ok(())
    }

    pub fn validate_manual_reconciliation(reason: &str) -> Result<(), BillingCockpitError> {
        if reason.trim().len() < 3 {
            return Err(BillingCockpitError::InvalidReconciliationReason);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strictly_rejects_live_stripe_credentials() {
        assert_eq!(
            BillingCockpitView::assert_test_mode("sk_live_12345"),
            Err(BillingCockpitError::LiveModeForbidden)
        );
        assert_eq!(
            BillingCockpitView::assert_test_mode("whsec_live_9999"),
            Err(BillingCockpitError::LiveModeForbidden)
        );
        assert!(BillingCockpitView::assert_test_mode("sk_test_12345").is_ok());
    }

    #[test]
    fn validates_manual_reconciliation_reason() {
        assert!(BillingCockpitView::validate_manual_reconciliation("Manual bank transfer verified").is_ok());
        assert_eq!(
            BillingCockpitView::validate_manual_reconciliation("no"),
            Err(BillingCockpitError::InvalidReconciliationReason)
        );
    }
}
