use chrono::Utc;
use uuid::Uuid;

use super::db;
use super::types::{
    EnterpriseTrustCenterResponse, TrustCenterAuditStatus, TrustCenterDocument, TrustCenterDomain,
    TrustCenterHostingRegion, TrustCenterMfaStatus, TrustCenterSsoProvider, TrustCenterSsoStatus,
    TrustCenterSubprocessor, TrustCenterTenant,
};
use crate::database::Database;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub async fn get_trust_center(
    pool: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseTrustCenterResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(pool, auth, tenant_id).await?;
    let tenant = db::tenant(pool, tenant_id).await?;
    let mfa = db::mfa(pool, tenant_id).await?;
    let providers = db::sso_providers(pool, tenant_id).await?;
    let domains = db::domains(pool, tenant_id).await?;
    let regions = db::hosting_regions(pool, tenant_id).await?;
    let audit_events = crate::domains::enterprise::db::list_audit_events(pool, tenant_id, 10)
        .await?
        .into_iter()
        .map(crate::domains::enterprise::db::AuditEventRow::into_view)
        .collect();

    let active_providers = providers
        .iter()
        .filter(|provider| provider.status == "active")
        .count() as i64;
    let required_domains = domains.iter().filter(|domain| domain.sso_required).count() as i64;

    Ok(EnterpriseTrustCenterResponse {
        tenant: TrustCenterTenant {
            id: tenant.id,
            name: tenant.name,
            slug: tenant.slug,
            status: tenant.status,
            security_tier: tenant.security_tier,
        },
        mfa: TrustCenterMfaStatus {
            enabled: mfa.members_with_mfa > 0,
            active_members: mfa.active_members,
            members_with_mfa: mfa.members_with_mfa,
            active_factors: mfa.active_factors,
            passkey_factors: mfa.passkey_factors,
        },
        sso: TrustCenterSsoStatus {
            enabled: active_providers > 0 || required_domains > 0,
            active_providers,
            required_domains,
            providers: providers
                .into_iter()
                .map(|provider| TrustCenterSsoProvider {
                    id: provider.id,
                    name: provider.name,
                    provider_type: provider.provider_type,
                    provider_family: provider.provider_family,
                    status: provider.status,
                    created_at: provider.created_at,
                })
                .collect(),
        },
        verified_domains: domains
            .into_iter()
            .map(|domain| TrustCenterDomain {
                id: domain.id,
                domain: domain.domain,
                verified: domain.verified_at.is_some(),
                sso_required: domain.sso_required,
                verified_at: domain.verified_at,
            })
            .collect(),
        audit: TrustCenterAuditStatus {
            immutable: true,
            recent_events: audit_events,
        },
        hosting_regions: regions
            .into_iter()
            .map(|region| TrustCenterHostingRegion {
                data_region: region.data_region,
                legal_jurisdiction: region.legal_jurisdiction,
                workspace_count: region.workspace_count,
            })
            .collect(),
        dpa: dpa_document(),
        subprocessors: subprocessors(),
        generated_at: Utc::now(),
    })
}

fn dpa_document() -> TrustCenterDocument {
    TrustCenterDocument {
        name: "Data Processing Agreement".to_string(),
        status: "ready_for_signature".to_string(),
        version: "2026-05-11".to_string(),
        url: "/legal/data-processing-agreement".to_string(),
    }
}

fn subprocessors() -> Vec<TrustCenterSubprocessor> {
    vec![
        subprocessor(
            "Scaleway",
            "Hebergement",
            "Compte, fichiers, metadata, logs",
            "France",
            false,
            "N/A",
        ),
        subprocessor(
            "Stripe",
            "Paiement et billing",
            "Facturation, TVA, transactions",
            "USA",
            true,
            "SCC",
        ),
        subprocessor(
            "PostHog",
            "Product analytics",
            "Events usage, metadata techniques",
            "USA",
            true,
            "SCC",
        ),
        subprocessor(
            "Sentry",
            "Error tracking",
            "Logs d'erreurs, metadata techniques",
            "USA",
            true,
            "SCC",
        ),
        subprocessor(
            "Grafana Labs",
            "Observabilite cloud",
            "Metriques, traces, logs rediges",
            "UE/USA",
            true,
            "DPA Grafana Cloud + SCC",
        ),
        subprocessor(
            "Cloudflare",
            "WAF, CDN, DNS",
            "IP, logs reseau, requetes HTTP",
            "Global edge",
            true,
            "SCC",
        ),
    ]
}

fn subprocessor(
    name: &str,
    service: &str,
    data_categories: &str,
    location: &str,
    transfer_outside_eea: bool,
    transfer_safeguard: &str,
) -> TrustCenterSubprocessor {
    TrustCenterSubprocessor {
        name: name.to_string(),
        service: service.to_string(),
        data_categories: data_categories.to_string(),
        location: location.to_string(),
        transfer_outside_eea,
        transfer_safeguard: transfer_safeguard.to_string(),
    }
}
