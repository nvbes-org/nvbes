use super::{StripeMappingRow, evaluate_stripe_mappings};

fn valid_row(code: &str, price: &str) -> StripeMappingRow {
    StripeMappingRow {
        plan_code: code.into(),
        stripe_price_id: price.into(),
        currency: "eur".into(),
        amount_cents: 990,
        billing_interval: "month".into(),
        is_active: true,
    }
}

#[test]
fn accepts_well_formed_active_plans() {
    let report = evaluate_stripe_mappings(&[
        valid_row("starter", "price_starter"),
        valid_row("pro", "price_pro"),
    ]);
    assert_eq!(report.plans_checked, 2);
    assert_eq!(report.active_plans, 2);
    assert!(report.failures.is_empty());
}

#[test]
fn reports_empty_catalog_and_no_active_plan() {
    let empty = evaluate_stripe_mappings(&[]);
    assert!(empty.failures.iter().any(|f| f.contains("no plans")));

    let inactive = evaluate_stripe_mappings(&[StripeMappingRow {
        is_active: false,
        ..valid_row("starter", "price_starter")
    }]);
    assert!(
        inactive
            .failures
            .iter()
            .any(|f| f.contains("no active plan"))
    );
}

#[test]
fn reports_field_validation_failures() {
    let rows = vec![
        StripeMappingRow {
            plan_code: " ".into(),
            stripe_price_id: "".into(),
            currency: "EURO".into(),
            amount_cents: -1,
            billing_interval: "week".into(),
            is_active: true,
        },
        StripeMappingRow {
            plan_code: "dup".into(),
            stripe_price_id: "not_a_price".into(),
            currency: "eur".into(),
            amount_cents: 100,
            billing_interval: "year".into(),
            is_active: true,
        },
        valid_row("a", "price_shared"),
        valid_row("b", "price_shared"),
    ];
    let report = evaluate_stripe_mappings(&rows);
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.contains("empty plan_code"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.contains("empty stripe_price_id"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.contains("must start with"))
    );
    assert!(report.failures.iter().any(|f| f.contains("duplicate")));
    assert!(report.failures.iter().any(|f| f.contains("negative")));
    assert!(report.failures.iter().any(|f| f.contains("currency")));
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.contains("billing_interval"))
    );
}
