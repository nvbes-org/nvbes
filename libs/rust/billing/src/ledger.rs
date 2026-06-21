use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerEntryType {
    Invoice,
    TaxLiability,
    Payment,
    Refund,
    CreditNote,
    WriteOff,
    CommercialCredit,
    Adjustment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_type: LedgerEntryType,
    pub account_code: String,
    pub amount_minor: i64,
    pub currency: String,
}

pub fn ledger_balances_to_zero(entries: &[LedgerEntry]) -> bool {
    entries.iter().map(|entry| entry.amount_minor).sum::<i64>() == 0
}

pub fn paid_invoice_entries(total_minor: i64, tax_minor: i64, currency: &str) -> Vec<LedgerEntry> {
    vec![
        LedgerEntry {
            entry_type: LedgerEntryType::Invoice,
            account_code: "accounts_receivable".to_string(),
            amount_minor: total_minor,
            currency: currency.to_string(),
        },
        LedgerEntry {
            entry_type: LedgerEntryType::TaxLiability,
            account_code: "tax_liability".to_string(),
            amount_minor: -tax_minor,
            currency: currency.to_string(),
        },
        LedgerEntry {
            entry_type: LedgerEntryType::Invoice,
            account_code: "revenue".to_string(),
            amount_minor: -(total_minor - tax_minor),
            currency: currency.to_string(),
        },
        LedgerEntry {
            entry_type: LedgerEntryType::Payment,
            account_code: "cash".to_string(),
            amount_minor: total_minor,
            currency: currency.to_string(),
        },
        LedgerEntry {
            entry_type: LedgerEntryType::Payment,
            account_code: "accounts_receivable".to_string(),
            amount_minor: -total_minor,
            currency: currency.to_string(),
        },
    ]
}

pub fn refund_entries(amount_minor: i64, currency: &str) -> Vec<LedgerEntry> {
    vec![
        LedgerEntry {
            entry_type: LedgerEntryType::Refund,
            account_code: "refunds".to_string(),
            amount_minor,
            currency: currency.to_string(),
        },
        LedgerEntry {
            entry_type: LedgerEntryType::Refund,
            account_code: "cash".to_string(),
            amount_minor: -amount_minor,
            currency: currency.to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_payment_balances_invoice_entries() {
        let entries = paid_invoice_entries(4_680, 780, "EUR");
        assert!(ledger_balances_to_zero(&entries));
    }

    #[test]
    fn partial_refund_balances_as_append_only_entries() {
        let entries = refund_entries(1_000, "EUR");
        assert!(ledger_balances_to_zero(&entries));
    }
}
