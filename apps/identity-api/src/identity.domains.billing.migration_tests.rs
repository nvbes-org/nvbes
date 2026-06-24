const BILLING_PLATFORM_MIGRATION: &str =
    include_str!("../migrations/0016_billing_platform_core.sql");
const REGIONAL_PRICE_MIGRATION: &str =
    include_str!("../migrations/0017_regional_price_mappings.sql");
const GEO_LOOKUP_MIGRATION: &str = include_str!("../migrations/0018_geo_lookup_relations.sql");
const GEO_REPUTATION_MIGRATION: &str = include_str!("../migrations/0019_geo_reputation_labels.sql");

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

#[test]
fn regional_price_migration_supports_country_and_region_fallbacks() {
    for needle in [
        "country_code CHAR(2)",
        "pricing_region TEXT",
        "amount_minor BIGINT",
        "idx_billing_provider_price_mappings_market",
        "idx_stripe_price_mappings_plan_market",
    ] {
        assert!(
            REGIONAL_PRICE_MIGRATION.contains(needle),
            "regional price migration missing {needle}"
        );
    }
}

#[test]
fn geo_lookup_migration_persists_sources_relations_events_and_evidence() {
    for table in [
        "geo_sources",
        "geo_ip_network_relations",
        "geo_personal_ip_ranges",
        "geo_lookup_events",
        "geo_lookup_evidence",
    ] {
        assert!(
            GEO_LOOKUP_MIGRATION.contains(&format!("CREATE TABLE IF NOT EXISTS {table}")),
            "missing geo lookup table {table}"
        );
    }
    for source in ["arin", "ripe", "apnic", "lacnic", "afrinic"] {
        assert!(
            GEO_LOOKUP_MIGRATION.contains(source),
            "missing rdap source {source}"
        );
    }
    assert!(GEO_LOOKUP_MIGRATION.contains("relation_key TEXT NOT NULL"));
    assert!(GEO_LOOKUP_MIGRATION.contains("(source_code, relation_key)"));
}

#[test]
fn geo_reputation_migration_persists_network_labels_and_scores() {
    for needle in [
        "network_kind",
        "risk_score",
        "risk_labels",
        "datacenter",
        "vpn",
        "proxy",
        "residential",
        "mobile",
        "idx_geo_personal_ip_ranges_kind_score",
        "idx_geo_lookup_events_kind_score",
        "ip_intelligence",
    ] {
        assert!(
            GEO_REPUTATION_MIGRATION.contains(needle),
            "geo reputation migration missing {needle}"
        );
    }
}

#[tokio::test]
async fn geo_lookup_persistence_round_trips_when_database_is_available() {
    use nvbes_region::geo::{
        GeoLookupRecordContext, GeoLookupRequest, GeoResolver, load_personal_geo_database_tx,
        record_geo_resolution_tx,
    };
    use sqlx::postgres::PgPoolOptions;

    let database_url =
        match std::env::var("DATABASE_URL").or_else(|_| std::env::var("NVBES_DATABASE_URL")) {
            Ok(value) => value,
            Err(_) => {
                eprintln!("skipping test: DATABASE_URL is not set");
                return;
            }
        };
    let pool = match PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("skipping test: database is not reachable: {error}");
            return;
        }
    };

    crate::test_support::ensure_test_database(&pool).await;
    let mut tx = pool.begin().await.expect("transaction should start");
    sqlx::query(
        r#"
        INSERT INTO geo_personal_ip_ranges (network, country_code, source_reference)
        VALUES ('203.0.113.0/24'::cidr, 'FR', 'test-fixture')
        ON CONFLICT (network)
        DO UPDATE SET country_code = EXCLUDED.country_code, enabled = TRUE
        "#,
    )
    .execute(&mut *tx)
    .await
    .expect("personal geo range should be inserted");

    let personal_database = load_personal_geo_database_tx(&mut tx)
        .await
        .expect("personal geo database should load");
    let resolution = GeoResolver::new(personal_database).resolve(GeoLookupRequest {
        ip: Some("203.0.113.42".parse().unwrap()),
        ..GeoLookupRequest::default()
    });
    assert_eq!(
        resolution
            .location
            .as_ref()
            .map(|location| location.country_code.as_str()),
        Some("FR")
    );

    let event_id = record_geo_resolution_tx(
        &mut tx,
        GeoLookupRecordContext {
            purpose: "payment",
            subject_type: Some("workspace"),
            subject_id: None,
            request_id: Some("test-geo-round-trip"),
        },
        &resolution,
    )
    .await
    .expect("geo resolution should be recorded");
    let evidence_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM geo_lookup_evidence WHERE lookup_event_id = $1")
            .bind(event_id)
            .fetch_one(&mut *tx)
            .await
            .expect("evidence count should be readable");
    assert!(evidence_count >= 1);

    tx.rollback()
        .await
        .expect("test transaction should roll back");
}

#[tokio::test]
async fn geo_maintenance_preserves_referenced_relations_when_database_is_available() {
    use nvbes_region::geo::run_geo_maintenance_tx;
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    let database_url =
        match std::env::var("DATABASE_URL").or_else(|_| std::env::var("NVBES_DATABASE_URL")) {
            Ok(value) => value,
            Err(_) => {
                eprintln!("skipping test: DATABASE_URL is not set");
                return;
            }
        };
    let pool = match PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("skipping test: database is not reachable: {error}");
            return;
        }
    };

    crate::test_support::ensure_test_database(&pool).await;
    let mut tx = pool.begin().await.expect("transaction should start");
    let suffix = Uuid::new_v4().as_u128();
    let personal_network = format!(
        "2001:db8:{:x}:{:x}::/64",
        suffix as u16,
        (suffix >> 16) as u16
    );
    let referenced_network = format!(
        "2001:db8:{:x}:{:x}::/64",
        (suffix >> 32) as u16,
        (suffix >> 48) as u16
    );
    let unreferenced_network = format!(
        "2001:db8:{:x}:{:x}::/64",
        (suffix >> 64) as u16,
        (suffix >> 80) as u16
    );

    let referenced_relation_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, country_code, expires_at
        )
        VALUES ('arin', $1, 'arin', $2::cidr, 'US', NOW() - INTERVAL '1 day')
        RETURNING id
        "#,
    )
    .bind(format!("test-referenced-{suffix}"))
    .bind(&referenced_network)
    .fetch_one(&mut *tx)
    .await
    .expect("referenced relation should be inserted");

    let unreferenced_relation_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO geo_ip_network_relations (
          source_code, relation_key, registry, network, country_code, expires_at
        )
        VALUES ('arin', $1, 'arin', $2::cidr, 'US', NOW() - INTERVAL '1 day')
        RETURNING id
        "#,
    )
    .bind(format!("test-unreferenced-{suffix}"))
    .bind(&unreferenced_network)
    .fetch_one(&mut *tx)
    .await
    .expect("unreferenced relation should be inserted");

    let event_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO geo_lookup_events (
          purpose, selected_source, confidence, private_network
        )
        VALUES ('security', 'arin', 'high', FALSE)
        RETURNING id
        "#,
    )
    .fetch_one(&mut *tx)
    .await
    .expect("lookup event should be inserted");

    sqlx::query(
        r#"
        INSERT INTO geo_lookup_evidence (
          lookup_event_id, source_code, network_relation_id, country_code,
          confidence, accepted, reason
        )
        VALUES ($1, 'arin', $2, 'US', 'high', TRUE, 'test')
        "#,
    )
    .bind(event_id)
    .bind(referenced_relation_id)
    .execute(&mut *tx)
    .await
    .expect("lookup evidence should be inserted");

    sqlx::query(
        r#"
        INSERT INTO geo_personal_ip_ranges (network, country_code, expires_at)
        VALUES ($1::cidr, 'FR', NOW() - INTERVAL '1 day')
        "#,
    )
    .bind(&personal_network)
    .execute(&mut *tx)
    .await
    .expect("expired personal range should be inserted");

    let report = run_geo_maintenance_tx(&mut tx)
        .await
        .expect("geo maintenance should run");

    assert_eq!(report.expired_personal_ranges_disabled, 1);
    assert_eq!(report.expired_unreferenced_relations_deleted, 1);

    let referenced_still_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM geo_ip_network_relations WHERE id = $1)")
            .bind(referenced_relation_id)
            .fetch_one(&mut *tx)
            .await
            .expect("referenced relation should be readable");
    assert!(referenced_still_exists);

    let unreferenced_still_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM geo_ip_network_relations WHERE id = $1)")
            .bind(unreferenced_relation_id)
            .fetch_one(&mut *tx)
            .await
            .expect("unreferenced relation should be readable");
    assert!(!unreferenced_still_exists);

    let personal_enabled: bool =
        sqlx::query_scalar("SELECT enabled FROM geo_personal_ip_ranges WHERE network = $1::cidr")
            .bind(&personal_network)
            .fetch_one(&mut *tx)
            .await
            .expect("personal range should be readable");
    assert!(!personal_enabled);

    tx.rollback()
        .await
        .expect("test transaction should roll back");
}
