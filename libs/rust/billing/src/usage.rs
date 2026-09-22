use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use crate::invoices::{InvoiceLine, InvoiceTotals, calculate_invoice_totals};
use crate::pricing::Money;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEvent {
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub meter_code: String,
    pub quantity: i64,
    pub unit: String,
    pub occurred_at: DateTime<Utc>,
    pub source: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageCorrection {
    pub meter_code: String,
    pub quantity_delta: i64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRollup {
    pub meter_code: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub quantity: i64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaBalance {
    pub quota_code: String,
    pub included_quantity: i64,
    pub used_quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProFormaInvoice {
    pub lines: Vec<InvoiceLine>,
    pub totals: InvoiceTotals,
    pub currency: String,
}

pub fn deduplicate_usage_events(events: Vec<UsageEvent>) -> Vec<UsageEvent> {
    let mut seen = BTreeSet::new();
    events
        .into_iter()
        .filter(|event| {
            seen.insert((
                event.tenant_id,
                event.source.clone(),
                event.idempotency_key.clone(),
            ))
        })
        .collect()
}

pub fn rollup_usage(
    events: &[UsageEvent],
    period_start: NaiveDate,
    period_end: NaiveDate,
) -> Vec<UsageRollup> {
    let mut totals: BTreeMap<(String, String), i64> = BTreeMap::new();
    for event in events {
        let event_date = event.occurred_at.date_naive();
        if event_date >= period_start && event_date < period_end {
            *totals
                .entry((event.meter_code.clone(), event.unit.clone()))
                .or_default() += event.quantity;
        }
    }

    totals
        .into_iter()
        .map(|((meter_code, unit), quantity)| UsageRollup {
            meter_code,
            period_start,
            period_end,
            quantity,
            unit,
        })
        .collect()
}

pub fn apply_correction(quantity: i64, correction: &UsageCorrection) -> i64 {
    (quantity + correction.quantity_delta).max(0)
}

pub fn storage_gb_month_billable_quantity(used_bytes: i64, included_bytes: i64) -> i64 {
    let billable_bytes = used_bytes.saturating_sub(included_bytes);
    if billable_bytes == 0 {
        0
    } else {
        (billable_bytes + 1024 * 1024 * 1024 - 1) / (1024 * 1024 * 1024)
    }
}

pub fn quota_balance(quota_code: &str, included_quantity: i64, used_quantity: i64) -> QuotaBalance {
    QuotaBalance {
        quota_code: quota_code.to_string(),
        included_quantity,
        used_quantity,
    }
}

pub fn overage_allowed(
    balance: &QuotaBalance,
    spend_cap_minor: Option<i64>,
    unit_price: &Money,
) -> bool {
    let overage_quantity = balance
        .used_quantity
        .saturating_sub(balance.included_quantity);
    let overage_amount = overage_quantity * unit_price.amount_minor;
    spend_cap_minor.is_none_or(|cap| overage_amount <= cap)
}

pub fn pro_forma_from_lines(lines: Vec<InvoiceLine>, currency: &str) -> ProFormaInvoice {
    let totals = calculate_invoice_totals(&lines);
    ProFormaInvoice {
        lines,
        totals,
        currency: currency.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage_event(idempotency_key: &str, quantity: i64) -> UsageEvent {
        UsageEvent {
            tenant_id: Uuid::nil(),
            workspace_id: None,
            meter_code: "storage_gb_month".to_string(),
            quantity,
            unit: "gb_month".to_string(),
            occurred_at: DateTime::from_timestamp(1_735_689_600, 0).unwrap(),
            source: "cloud-service".to_string(),
            idempotency_key: idempotency_key.to_string(),
        }
    }

    #[test]
    fn duplicate_usage_event_is_ignored() {
        let events =
            deduplicate_usage_events(vec![usage_event("same", 10), usage_event("same", 20)]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].quantity, 10);
    }

    #[test]
    fn negative_correction_is_compensated_without_going_below_zero() {
        let corrected = apply_correction(
            5,
            &UsageCorrection {
                meter_code: "storage_gb_month".to_string(),
                quantity_delta: -10,
                reason: "bad snapshot".to_string(),
            },
        );
        assert_eq!(corrected, 0);
    }

    #[test]
    fn storage_overage_rounds_up_to_gb_month() {
        let included = 10 * 1024 * 1024 * 1024;
        let used = included + 1;

        assert_eq!(storage_gb_month_billable_quantity(used, included), 1);
    }

    #[test]
    fn spend_cap_blocks_unauthorized_overage() {
        let balance = quota_balance("storage_gb", 100, 125);
        assert!(!overage_allowed(
            &balance,
            Some(50),
            &crate::pricing::Money::eur(4)
        ));
        assert!(overage_allowed(
            &balance,
            Some(100),
            &crate::pricing::Money::eur(4)
        ));
    }

    #[test]
    fn pro_forma_uses_invoice_total_engine() {
        let pro_forma = pro_forma_from_lines(
            vec![InvoiceLine {
                line_type: "usage".to_string(),
                description: "Storage overage".to_string(),
                quantity: 2,
                unit_amount_minor: 4,
                amount_minor: 8,
                tax_minor: 2,
            }],
            "EUR",
        );

        assert_eq!(pro_forma.totals.total_minor, 10);
    }
}
