use chrono::{DateTime, NaiveDate};
use uuid::Uuid;

use super::{
    UsageCorrection, UsageEvent, apply_correction, deduplicate_usage_events, overage_allowed,
    pro_forma_from_lines, quota_balance, rollup_usage, storage_gb_month_billable_quantity,
};
use crate::invoices::InvoiceLine;

fn usage_event(
    idempotency_key: &str,
    quantity: i64,
    occurred_at: DateTime<chrono::Utc>,
) -> UsageEvent {
    UsageEvent {
        tenant_id: Uuid::nil(),
        workspace_id: None,
        meter_code: "storage_gb_month".to_string(),
        quantity,
        unit: "gb_month".to_string(),
        occurred_at,
        source: "cloud-service".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}

#[test]
fn duplicate_usage_event_is_ignored() {
    let ts = DateTime::from_timestamp(1_735_689_600, 0).unwrap();
    let events = deduplicate_usage_events(vec![
        usage_event("same", 10, ts),
        usage_event("same", 20, ts),
    ]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].quantity, 10);
}

#[test]
fn rollup_usage_filters_by_period_and_groups_meters() {
    let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
    let inside = DateTime::from_timestamp(1_735_689_600, 0).unwrap();
    let outside = DateTime::from_timestamp(1_704_067_200, 0).unwrap();
    let rollups = rollup_usage(
        &[
            usage_event("a", 5, inside),
            usage_event("b", 3, inside),
            usage_event("c", 99, outside),
        ],
        start,
        end,
    );
    assert_eq!(rollups.len(), 1);
    assert_eq!(rollups[0].quantity, 8);
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
    assert_eq!(storage_gb_month_billable_quantity(included, included), 0);
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
    assert!(overage_allowed(
        &balance,
        None,
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
    assert_eq!(pro_forma.currency, "EUR");
}
