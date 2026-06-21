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
mod tests {
    use super::*;
    use crate::ledger::{LedgerEntry, LedgerEntryType};

    #[test]
    fn provider_amount_mismatch_produces_actionable_alert() {
        let alert = reconcile_payment(PaymentReconciliationInput {
            provider_payment_id: "pi_1".to_string(),
            provider_amount_minor: 1_000,
            provider_currency: "EUR".to_string(),
            provider_status: PaymentStatus::Captured,
            internal_amount_minor: Some(900),
            internal_currency: Some("EUR".to_string()),
            internal_status: Some(PaymentStatus::Captured),
        });

        assert!(alert.actionable);
        assert_eq!(
            alert.differences[0].difference_type,
            ReconciliationDifferenceType::AmountMismatch
        );
    }

    #[test]
    fn unbalanced_ledger_is_detected() {
        let diff = detect_unbalanced_ledger(&[LedgerEntry {
            entry_type: LedgerEntryType::Invoice,
            account_code: "accounts_receivable".to_string(),
            amount_minor: 100,
            currency: "EUR".to_string(),
        }])
        .expect("unbalanced ledger should create diff");

        assert_eq!(
            diff.difference_type,
            ReconciliationDifferenceType::UnbalancedLedger
        );
    }
}
