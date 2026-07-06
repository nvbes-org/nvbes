use sqlx::Row;
use tonic::Status;

use crate::grpc::pb::nvbes::billing::v1::{
    AdminBillingPlatformCenterSnapshot, BillingProviderRoutingRule, BillingProviderSummary,
    EinvoicingProfile, KycProfile, ProviderMigrationRun, RegionPolicy,
};

pub async fn billing_platform_center(
    db: &sqlx::PgPool,
) -> Result<AdminBillingPlatformCenterSnapshot, Status> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM billing_providers WHERE status = 'active') AS active_provider_count,
          (
            SELECT COUNT(*) FROM billing_provider_accounts
            WHERE status = 'active'
          ) AS active_provider_account_count,
          (
            SELECT COUNT(*) FROM billing_provider_routing_rules
            WHERE status = 'active'
          ) AS active_routing_rule_count,
          (
            SELECT COUNT(*) FROM billing_provider_routing_rules
            WHERE status = 'active' AND fallback_enabled = TRUE
          ) AS fallback_routing_rule_count,
          (
            SELECT COUNT(*) FROM billing_provider_migration_runs
            WHERE status IN ('planned', 'running')
          ) AS planned_migration_count,
          (
            SELECT COUNT(*) FROM billing_kyc_profiles
            WHERE proof_reference IS NULL
          ) AS pending_kyc_profile_count,
          (SELECT COUNT(*) FROM billing_region_policies) AS region_policy_count,
          (
            SELECT COUNT(*) FROM billing_einvoicing_profiles
            WHERE status = 'active'
          ) AS active_einvoicing_profile_count
        "#,
    )
    .fetch_one(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(AdminBillingPlatformCenterSnapshot {
        active_provider_count: metrics.get("active_provider_count"),
        active_provider_account_count: metrics.get("active_provider_account_count"),
        active_routing_rule_count: metrics.get("active_routing_rule_count"),
        fallback_routing_rule_count: metrics.get("fallback_routing_rule_count"),
        planned_migration_count: metrics.get("planned_migration_count"),
        pending_kyc_profile_count: metrics.get("pending_kyc_profile_count"),
        region_policy_count: metrics.get("region_policy_count"),
        active_einvoicing_profile_count: metrics.get("active_einvoicing_profile_count"),
        providers: load_providers(db).await?,
        routing_rules: load_routing_rules(db).await?,
        provider_migrations: load_provider_migrations(db).await?,
        kyc_profiles: load_kyc_profiles(db).await?,
        region_policies: load_region_policies(db).await?,
        einvoicing_profiles: load_einvoicing_profiles(db).await?,
    })
}

async fn load_providers(db: &sqlx::PgPool) -> Result<Vec<BillingProviderSummary>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT bp.provider::text AS provider, bp.status, COUNT(bpa.id) AS account_count,
          bp.updated_at
        FROM billing_providers bp
        LEFT JOIN billing_provider_accounts bpa ON bpa.provider = bp.provider
        GROUP BY bp.provider, bp.status, bp.updated_at
        ORDER BY bp.provider ASC
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| BillingProviderSummary {
            provider: row.get("provider"),
            status: row.get("status"),
            account_count: row.get("account_count"),
            updated_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                .to_rfc3339(),
        })
        .collect())
}

async fn load_routing_rules(db: &sqlx::PgPool) -> Result<Vec<BillingProviderRoutingRule>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT id, priority, provider::text AS provider, country::text AS country,
          currency::text AS currency, payment_method, customer_type,
          min_amount_minor, max_amount_minor, fallback_enabled, status, created_at, updated_at
        FROM billing_provider_routing_rules
        ORDER BY priority ASC, updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let min_amount_minor = row.get::<Option<i64>, _>("min_amount_minor");
            let max_amount_minor = row.get::<Option<i64>, _>("max_amount_minor");
            BillingProviderRoutingRule {
                id: row.get::<uuid::Uuid, _>("id").to_string(),
                priority: row.get("priority"),
                provider: row.get("provider"),
                country: row.get::<Option<String>, _>("country").unwrap_or_default(),
                currency: row.get::<Option<String>, _>("currency").unwrap_or_default(),
                payment_method: row
                    .get::<Option<String>, _>("payment_method")
                    .unwrap_or_default(),
                customer_type: row
                    .get::<Option<String>, _>("customer_type")
                    .unwrap_or_default(),
                min_amount_minor: min_amount_minor.unwrap_or_default(),
                max_amount_minor: max_amount_minor.unwrap_or_default(),
                has_min_amount_minor: min_amount_minor.is_some(),
                has_max_amount_minor: max_amount_minor.is_some(),
                fallback_enabled: row.get("fallback_enabled"),
                status: row.get("status"),
                created_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
                updated_at: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                    .to_rfc3339(),
            }
        })
        .collect())
}

async fn load_provider_migrations(db: &sqlx::PgPool) -> Result<Vec<ProviderMigrationRun>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT pm.id, pm.tenant_id, t.name AS tenant_name, pm.from_provider::text AS from_provider,
          pm.to_provider::text AS to_provider, pm.status, pm.started_at, pm.updated_at
        FROM billing_provider_migration_runs pm
        JOIN tenants t ON t.id = pm.tenant_id
        ORDER BY pm.updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| ProviderMigrationRun {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            from_provider: row.get("from_provider"),
            to_provider: row.get("to_provider"),
            status: row.get("status"),
            started_at: row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at")
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
            updated_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                .to_rfc3339(),
        })
        .collect())
}

async fn load_kyc_profiles(db: &sqlx::PgPool) -> Result<Vec<KycProfile>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT kyc.id, kyc.tenant_id, t.name AS tenant_name, kyc.company_name,
          kyc.company_domain, kyc.vat_id, kyc.proof_reference,
          kyc.review_status, kyc.updated_at
        FROM billing_kyc_profiles kyc
        JOIN tenants t ON t.id = kyc.tenant_id
        ORDER BY (kyc.review_status = 'pending') DESC, kyc.updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| KycProfile {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            company_name: row
                .get::<Option<String>, _>("company_name")
                .unwrap_or_default(),
            company_domain: row
                .get::<Option<String>, _>("company_domain")
                .unwrap_or_default(),
            vat_id: row.get::<Option<String>, _>("vat_id").unwrap_or_default(),
            proof_reference: row
                .get::<Option<String>, _>("proof_reference")
                .unwrap_or_default(),
            review_status: row.get("review_status"),
            updated_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                .to_rfc3339(),
        })
        .collect())
}

async fn load_region_policies(db: &sqlx::PgPool) -> Result<Vec<RegionPolicy>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT id, country::text AS country, currency::text AS currency, allowed_payment_methods,
          invoice_retention_years, tax_evidence_required, einvoicing_profile_code
        FROM billing_region_policies
        ORDER BY country ASC, currency ASC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| RegionPolicy {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            country: row.get("country"),
            currency: row.get("currency"),
            allowed_payment_methods: row.get("allowed_payment_methods"),
            invoice_retention_years: row.get("invoice_retention_years"),
            tax_evidence_required: row.get("tax_evidence_required"),
            einvoicing_profile_code: row
                .get::<Option<String>, _>("einvoicing_profile_code")
                .unwrap_or_default(),
        })
        .collect())
}

async fn load_einvoicing_profiles(db: &sqlx::PgPool) -> Result<Vec<EinvoicingProfile>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT id, code, country::text AS country, format, status, updated_at
        FROM billing_einvoicing_profiles
        ORDER BY updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| EinvoicingProfile {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            code: row.get("code"),
            country: row.get::<Option<String>, _>("country").unwrap_or_default(),
            format: row.get("format"),
            status: row.get("status"),
            updated_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                .to_rfc3339(),
        })
        .collect())
}
