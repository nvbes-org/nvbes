use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct BillingPlatformSnapshot {
    active_provider_count: i64,
    active_provider_account_count: i64,
    active_routing_rule_count: i64,
    fallback_routing_rule_count: i64,
    planned_migration_count: i64,
    pending_kyc_profile_count: i64,
    region_policy_count: i64,
    active_einvoicing_profile_count: i64,
    providers: Vec<BillingProviderSummary>,
    routing_rules: Vec<ProviderRoutingRule>,
    provider_migrations: Vec<ProviderMigrationRun>,
    kyc_profiles: Vec<KycProfile>,
    region_policies: Vec<RegionPolicy>,
    einvoicing_profiles: Vec<EinvoicingProfile>,
}

#[derive(Debug, Serialize)]
struct BillingProviderSummary {
    provider: String,
    status: String,
    account_count: i64,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProviderRoutingRule {
    id: Uuid,
    priority: i32,
    provider: String,
    country: Option<String>,
    currency: Option<String>,
    payment_method: Option<String>,
    customer_type: Option<String>,
    fallback_enabled: bool,
    status: String,
}

#[derive(Debug, Serialize)]
struct ProviderMigrationRun {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    from_provider: String,
    to_provider: String,
    status: String,
    started_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct KycProfile {
    id: Uuid,
    tenant_id: Uuid,
    tenant_name: String,
    company_name: Option<String>,
    company_domain: Option<String>,
    vat_id: Option<String>,
    proof_reference: Option<String>,
    review_status: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RegionPolicy {
    id: Uuid,
    country: String,
    currency: String,
    allowed_payment_methods: Vec<String>,
    invoice_retention_years: i32,
    tax_evidence_required: bool,
    einvoicing_profile_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct EinvoicingProfile {
    id: Uuid,
    code: String,
    country: Option<String>,
    format: String,
    status: String,
    updated_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/billing-platform-center",
        get(billing_platform_center_route),
    )
}

async fn billing_platform_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BillingPlatformSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_billing_platform(&state.db).await?))
}

async fn load_billing_platform(db: &PgPool) -> Result<BillingPlatformSnapshot, AppError> {
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
    .await?;

    Ok(BillingPlatformSnapshot {
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

async fn load_providers(db: &PgPool) -> Result<Vec<BillingProviderSummary>, AppError> {
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
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| BillingProviderSummary {
            provider: row.get("provider"),
            status: row.get("status"),
            account_count: row.get("account_count"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_routing_rules(db: &PgPool) -> Result<Vec<ProviderRoutingRule>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, priority, provider::text AS provider, country::text AS country,
          currency::text AS currency, payment_method, customer_type, fallback_enabled, status
        FROM billing_provider_routing_rules
        ORDER BY priority ASC, updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ProviderRoutingRule {
            id: row.get("id"),
            priority: row.get("priority"),
            provider: row.get("provider"),
            country: row.get("country"),
            currency: row.get("currency"),
            payment_method: row.get("payment_method"),
            customer_type: row.get("customer_type"),
            fallback_enabled: row.get("fallback_enabled"),
            status: row.get("status"),
        })
        .collect())
}

async fn load_provider_migrations(db: &PgPool) -> Result<Vec<ProviderMigrationRun>, AppError> {
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
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ProviderMigrationRun {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            from_provider: row.get("from_provider"),
            to_provider: row.get("to_provider"),
            status: row.get("status"),
            started_at: row.get("started_at"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_kyc_profiles(db: &PgPool) -> Result<Vec<KycProfile>, AppError> {
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
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| KycProfile {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            tenant_name: row.get("tenant_name"),
            company_name: row.get("company_name"),
            company_domain: row.get("company_domain"),
            vat_id: row.get("vat_id"),
            proof_reference: row.get("proof_reference"),
            review_status: row.get("review_status"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_region_policies(db: &PgPool) -> Result<Vec<RegionPolicy>, AppError> {
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
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RegionPolicy {
            id: row.get("id"),
            country: row.get("country"),
            currency: row.get("currency"),
            allowed_payment_methods: row.get("allowed_payment_methods"),
            invoice_retention_years: row.get("invoice_retention_years"),
            tax_evidence_required: row.get("tax_evidence_required"),
            einvoicing_profile_code: row.get("einvoicing_profile_code"),
        })
        .collect())
}

async fn load_einvoicing_profiles(db: &PgPool) -> Result<Vec<EinvoicingProfile>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, code, country::text AS country, format, status, updated_at
        FROM billing_einvoicing_profiles
        ORDER BY updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| EinvoicingProfile {
            id: row.get("id"),
            code: row.get("code"),
            country: row.get("country"),
            format: row.get("format"),
            status: row.get("status"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}
