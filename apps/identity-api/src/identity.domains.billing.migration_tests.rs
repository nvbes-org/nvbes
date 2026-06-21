const BILLING_PLATFORM_MIGRATION: &str =
    include_str!("../migrations/0016_billing_platform_core.sql");

#[test]
fn billing_platform_migration_covers_canonical_table_groups() {
    for table in [
        "billing_products",
        "billing_plans",
        "billing_plan_versions",
        "billing_price_versions",
        "billing_customer_profiles",
        "billing_tax_profiles",
        "billing_provider_customers",
        "billing_provider_mappings",
        "billing_provider_price_mappings",
        "billing_provider_events",
        "billing_usage_events",
        "billing_usage_rollups",
        "billing_entitlement_snapshots",
        "billing_subscriptions",
        "billing_invoices",
        "billing_invoice_lines",
        "billing_payments",
        "billing_refunds",
        "billing_ledger_entries",
        "billing_dunning_cases",
        "billing_reconciliation_runs",
        "billing_export_runs",
        "billing_region_policies",
        "billing_einvoicing_profiles",
    ] {
        assert!(
            BILLING_PLATFORM_MIGRATION.contains(&format!("CREATE TABLE IF NOT EXISTS {table}")),
            "missing canonical billing table {table}"
        );
    }
}

#[test]
fn billing_platform_migration_seeds_supported_providers() {
    assert!(BILLING_PLATFORM_MIGRATION.contains("ADD VALUE IF NOT EXISTS 'mollie'"));
    assert!(BILLING_PLATFORM_MIGRATION.contains("VALUES ('stripe'), ('mollie')"));
}

#[test]
fn billing_platform_migration_preserves_stripe_compatibility_mappings() {
    assert!(BILLING_PLATFORM_MIGRATION.contains("FROM stripe_price_mappings"));
    assert!(BILLING_PLATFORM_MIGRATION.contains("billing_legacy_stripe_price_mappings"));
    assert!(BILLING_PLATFORM_MIGRATION.contains("provider_product_id AS stripe_product_id"));
    assert!(BILLING_PLATFORM_MIGRATION.contains("provider_price_id AS stripe_price_id"));
}

#[test]
fn billing_platform_migration_supports_financial_ledger_lifecycle() {
    for entry_type in [
        "invoice",
        "tax_liability",
        "payment",
        "refund",
        "credit_note",
        "write_off",
        "commercial_credit",
        "adjustment",
    ] {
        assert!(
            BILLING_PLATFORM_MIGRATION.contains(&format!("'{entry_type}'")),
            "missing ledger entry type {entry_type}"
        );
    }
    assert!(BILLING_PLATFORM_MIGRATION.contains("amount_minor BIGINT NOT NULL"));
}
