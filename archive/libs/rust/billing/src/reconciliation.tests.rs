use super::{
    PaymentReconciliationInput, ReconciliationDifferenceType, detect_amount_mismatch,
    detect_unbalanced_ledger, reconcile_payment,
};
use crate::ledger::{LedgerEntry, LedgerEntryType};
use crate::payments::PaymentStatus;

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
fn missing_internal_payment_is_actionable() {
    let alert = reconcile_payment(PaymentReconciliationInput {
        provider_payment_id: "pi_missing".to_string(),
        provider_amount_minor: 500,
        provider_currency: "EUR".to_string(),
        provider_status: PaymentStatus::Captured,
        internal_amount_minor: None,
        internal_currency: None,
        internal_status: None,
    });
    assert!(alert.actionable);
    assert_eq!(
        alert.differences[0].difference_type,
        ReconciliationDifferenceType::MissingProviderEvent
    );
}

#[test]
fn currency_and_status_mismatches_are_reported() {
    let alert = reconcile_payment(PaymentReconciliationInput {
        provider_payment_id: "pi_2".to_string(),
        provider_amount_minor: 1_000,
        provider_currency: "EUR".to_string(),
        provider_status: PaymentStatus::Captured,
        internal_amount_minor: Some(1_000),
        internal_currency: Some("USD".to_string()),
        internal_status: Some(PaymentStatus::Failed),
    });
    assert!(alert.actionable);
    assert!(
        alert
            .differences
            .iter()
            .any(|diff| { diff.difference_type == ReconciliationDifferenceType::CurrencyMismatch })
    );
    assert!(
        alert
            .differences
            .iter()
            .any(|diff| { diff.difference_type == ReconciliationDifferenceType::StatusMismatch })
    );
}

#[test]
fn detect_amount_mismatch_is_none_when_amounts_match() {
    assert!(detect_amount_mismatch(100, 100).is_none());
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
