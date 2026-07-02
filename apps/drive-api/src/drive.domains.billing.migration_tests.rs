const INITIAL_SCHEMA_MIGRATION: &str = include_str!("../migrations/0001_initial_schema.sql");
const PROVIDER_CUSTOMERS_MIGRATION: &str =
    include_str!("../migrations/0007_billing_provider_customers.sql");
const PROVIDER_PRICE_MAPPINGS_MIGRATION: &str =
    include_str!("../migrations/0008_billing_provider_price_mappings.sql");
const PROVIDER_MOLLIE_MIGRATION: &str =
    include_str!("../migrations/0009_billing_provider_mollie.sql");

#[test]
fn drive_billing_migrations_open_provider_catalog() {
    assert!(INITIAL_SCHEMA_MIGRATION.contains("CREATE TYPE billing_provider AS ENUM ('stripe')"));
    assert!(
        PROVIDER_MOLLIE_MIGRATION
            .contains("ALTER TYPE billing_provider ADD VALUE IF NOT EXISTS 'mollie'")
    );
}

#[test]
fn drive_billing_migrations_backfill_provider_customers() {
    for needle in [
        "CREATE TABLE IF NOT EXISTS billing_provider_customers",
        "provider_customer_id TEXT NOT NULL",
        "UNIQUE (provider, provider_customer_id)",
        "INSERT INTO billing_provider_customers",
        "stripe_customer_id",
        "ON CONFLICT (provider, provider_customer_id) DO NOTHING",
    ] {
        assert!(
            PROVIDER_CUSTOMERS_MIGRATION.contains(needle),
            "drive provider customer migration missing {needle}"
        );
    }
}

#[test]
fn drive_billing_migrations_backfill_provider_price_mappings() {
    for needle in [
        "CREATE TABLE IF NOT EXISTS billing_provider_price_mappings",
        "provider_product_id TEXT NOT NULL",
        "provider_price_id TEXT NOT NULL",
        "UNIQUE (provider, provider_price_id)",
        "idx_billing_provider_price_mappings_plan_market",
        "INSERT INTO billing_provider_price_mappings",
        "SELECT plan_id",
        "stripe_product_id",
        "stripe_price_id",
        "ON CONFLICT (provider, provider_price_id) DO NOTHING",
    ] {
        assert!(
            PROVIDER_PRICE_MAPPINGS_MIGRATION.contains(needle),
            "drive provider price migration missing {needle}"
        );
    }
}
