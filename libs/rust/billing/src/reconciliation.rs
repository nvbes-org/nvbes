use serde::{Deserialize, Serialize};

use crate::ledger::{LedgerEntry, ledger_balances_to_zero};
use crate::payments::PaymentStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationDifferenceType {
    MissingProviderEvent,
    AmountMismatch,
    CurrencyMismatch,
    StatusMismatch,
    DuplicateCapture,
    UnbalancedLedger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationDifference {
    pub difference_type: ReconciliationDifferenceType,
    pub provider_reference: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentReconciliationInput {
    pub provider_payment_id: String,
    pub provider_amount_minor: i64,
    pub provider_currency: String,
    pub provider_status: PaymentStatus,
    pub internal_amount_minor: Option<i64>,
    pub internal_currency: Option<String>,
    pub internal_status: Option<PaymentStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationAlert {
    pub actionable: bool,
    pub differences: Vec<ReconciliationDifference>,
}

pub fn detect_amount_mismatch(
    provider_amount_minor: i64,
    internal_amount_minor: i64,
) -> Option<ReconciliationDifference> {
    (provider_amount_minor != internal_amount_minor).then(|| ReconciliationDifference {
        difference_type: ReconciliationDifferenceType::AmountMismatch,
        provider_reference: None,
        details: format!("provider={provider_amount_minor} internal={internal_amount_minor}"),
    })
}

pub fn reconcile_payment(input: PaymentReconciliationInput) -> ReconciliationAlert {
    let mut differences = Vec::new();

    let Some(internal_amount_minor) = input.internal_amount_minor else {
        differences.push(ReconciliationDifference {
            difference_type: ReconciliationDifferenceType::MissingProviderEvent,
            provider_reference: Some(input.provider_payment_id),
            details: "provider payment has no internal payment".to_string(),
        });
        return ReconciliationAlert {
            actionable: true,
            differences,
        };
    };

    if let Some(diff) = detect_amount_mismatch(input.provider_amount_minor, internal_amount_minor) {
        differences.push(ReconciliationDifference {
            provider_reference: Some(input.provider_payment_id.clone()),
            ..diff
        });
    }

    if input.internal_currency.as_deref() != Some(input.provider_currency.as_str()) {
        differences.push(ReconciliationDifference {
            difference_type: ReconciliationDifferenceType::CurrencyMismatch,
            provider_reference: Some(input.provider_payment_id.clone()),
            details: format!(
                "provider={} internal={}",
                input.provider_currency,
                input
                    .internal_currency
                    .unwrap_or_else(|| "missing".to_string())
            ),
        });
    }

    if input.internal_status != Some(input.provider_status) {
        differences.push(ReconciliationDifference {
            difference_type: ReconciliationDifferenceType::StatusMismatch,
            provider_reference: Some(input.provider_payment_id),
            details: "provider status differs from internal status".to_string(),
        });
    }

    ReconciliationAlert {
        actionable: !differences.is_empty(),
        differences,
    }
}

pub fn detect_unbalanced_ledger(entries: &[LedgerEntry]) -> Option<ReconciliationDifference> {
    (!ledger_balances_to_zero(entries)).then(|| ReconciliationDifference {
        difference_type: ReconciliationDifferenceType::UnbalancedLedger,
        provider_reference: None,
        details: "ledger entries do not balance to zero".to_string(),
    })
}

#[cfg(test)]
#[path = "reconciliation.tests.rs"]
mod tests;
