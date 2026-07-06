use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1 as billing_pb;

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
    min_amount_minor: Option<i64>,
    max_amount_minor: Option<i64>,
    fallback_enabled: bool,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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
    let actor_principal_id = actor_principal_id(&headers)?;
    let snapshot = crate::billing_grpc::get_admin_billing_platform_center(
        &state.billing_grpc_endpoint,
        BackofficeAccess {
            tenant_id: Uuid::nil(),
            actor_principal_id,
        },
    )
    .await?;
    Ok(Json(billing_platform_snapshot_from_grpc(snapshot)?))
}

fn billing_platform_snapshot_from_grpc(
    value: billing_pb::AdminBillingPlatformCenterSnapshot,
) -> Result<BillingPlatformSnapshot, AppError> {
    Ok(BillingPlatformSnapshot {
        active_provider_count: value.active_provider_count,
        active_provider_account_count: value.active_provider_account_count,
        active_routing_rule_count: value.active_routing_rule_count,
        fallback_routing_rule_count: value.fallback_routing_rule_count,
        planned_migration_count: value.planned_migration_count,
        pending_kyc_profile_count: value.pending_kyc_profile_count,
        region_policy_count: value.region_policy_count,
        active_einvoicing_profile_count: value.active_einvoicing_profile_count,
        providers: value
            .providers
            .into_iter()
            .map(provider_summary_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        routing_rules: value
            .routing_rules
            .into_iter()
            .map(routing_rule_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        provider_migrations: value
            .provider_migrations
            .into_iter()
            .map(provider_migration_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        kyc_profiles: value
            .kyc_profiles
            .into_iter()
            .map(kyc_profile_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        region_policies: value
            .region_policies
            .into_iter()
            .map(region_policy_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
        einvoicing_profiles: value
            .einvoicing_profiles
            .into_iter()
            .map(einvoicing_profile_from_grpc)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn provider_summary_from_grpc(
    value: billing_pb::BillingProviderSummary,
) -> Result<BillingProviderSummary, AppError> {
    Ok(BillingProviderSummary {
        provider: value.provider,
        status: value.status,
        account_count: value.account_count,
        updated_at: parse_datetime(&value.updated_at, "provider updated_at")?,
    })
}

fn routing_rule_from_grpc(
    value: billing_pb::BillingProviderRoutingRule,
) -> Result<ProviderRoutingRule, AppError> {
    Ok(ProviderRoutingRule {
        id: parse_uuid(&value.id, "routing rule id")?,
        priority: value.priority,
        provider: value.provider,
        country: empty_to_none(value.country),
        currency: empty_to_none(value.currency),
        payment_method: empty_to_none(value.payment_method),
        customer_type: empty_to_none(value.customer_type),
        min_amount_minor: value.has_min_amount_minor.then_some(value.min_amount_minor),
        max_amount_minor: value.has_max_amount_minor.then_some(value.max_amount_minor),
        fallback_enabled: value.fallback_enabled,
        status: value.status,
        created_at: parse_datetime(&value.created_at, "routing rule created_at")?,
        updated_at: parse_datetime(&value.updated_at, "routing rule updated_at")?,
    })
}

fn provider_migration_from_grpc(
    value: billing_pb::ProviderMigrationRun,
) -> Result<ProviderMigrationRun, AppError> {
    Ok(ProviderMigrationRun {
        id: parse_uuid(&value.id, "provider migration id")?,
        tenant_id: parse_uuid(&value.tenant_id, "provider migration tenant_id")?,
        tenant_name: value.tenant_name,
        from_provider: value.from_provider,
        to_provider: value.to_provider,
        status: value.status,
        started_at: parse_optional_datetime(&value.started_at, "provider migration started_at")?,
        updated_at: parse_datetime(&value.updated_at, "provider migration updated_at")?,
    })
}

fn kyc_profile_from_grpc(value: billing_pb::KycProfile) -> Result<KycProfile, AppError> {
    Ok(KycProfile {
        id: parse_uuid(&value.id, "kyc profile id")?,
        tenant_id: parse_uuid(&value.tenant_id, "kyc profile tenant_id")?,
        tenant_name: value.tenant_name,
        company_name: empty_to_none(value.company_name),
        company_domain: empty_to_none(value.company_domain),
        vat_id: empty_to_none(value.vat_id),
        proof_reference: empty_to_none(value.proof_reference),
        review_status: value.review_status,
        updated_at: parse_datetime(&value.updated_at, "kyc profile updated_at")?,
    })
}

fn region_policy_from_grpc(value: billing_pb::RegionPolicy) -> Result<RegionPolicy, AppError> {
    Ok(RegionPolicy {
        id: parse_uuid(&value.id, "region policy id")?,
        country: value.country,
        currency: value.currency,
        allowed_payment_methods: value.allowed_payment_methods,
        invoice_retention_years: value.invoice_retention_years,
        tax_evidence_required: value.tax_evidence_required,
        einvoicing_profile_code: empty_to_none(value.einvoicing_profile_code),
    })
}

fn einvoicing_profile_from_grpc(
    value: billing_pb::EinvoicingProfile,
) -> Result<EinvoicingProfile, AppError> {
    Ok(EinvoicingProfile {
        id: parse_uuid(&value.id, "einvoicing profile id")?,
        code: value.code,
        country: empty_to_none(value.country),
        format: value.format,
        status: value.status,
        updated_at: parse_datetime(&value.updated_at, "einvoicing profile updated_at")?,
    })
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::internal("billing_grpc_decode", field))
}

fn parse_optional_datetime(
    value: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_datetime(value, field).map(Some)
    }
}

fn empty_to_none(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
