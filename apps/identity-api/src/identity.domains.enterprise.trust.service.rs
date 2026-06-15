use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::db;
use super::types::{
    EnterpriseTrustCenterResponse, TrustCenterAuditStatus, TrustCenterDocument, TrustCenterDomain,
    TrustCenterHostingRegion, TrustCenterMfaStatus, TrustCenterSsoProvider, TrustCenterSsoStatus,
    TrustCenterSubprocessor, TrustCenterTenant,
};
use crate::database::Database;
use crate::domains::enterprise::types::EnterpriseAuditEvent;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

use crate::domains::authz::{AdminScope, resolve_admin_scope};

pub async fn get_trust_center(
    pool: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
) -> Result<EnterpriseTrustCenterResponse, AppError> {
    crate::domains::authz::ensure_tenant_management_access(pool, auth, tenant_id).await?;
    let scope = resolve_admin_scope(pool, auth, tenant_id, auth.organization_id).await?;
    if !matches!(scope, AdminScope::Tenant) {
        return Err(AppError::forbidden(
            "tenant_scope_required",
            "This action requires tenant-wide administrative privileges.",
        ));
    }
    let tenant = db::tenant(pool, tenant_id).await?;
    let mfa = db::mfa(pool, tenant_id).await?;
    let providers = db::sso_providers(pool, tenant_id).await?;
    let domains = db::domains(pool, tenant_id).await?;
    let regions = db::hosting_regions(pool, tenant_id).await?;
    let audit_events = crate::domains::enterprise::db::list_audit_events(pool, tenant_id, scope, 10)
        .await?
        .into_iter()
        .map(crate::domains::enterprise::db::AuditEventRow::into_view)
        .collect();

    Ok(build_trust_center_response(
        tenant,
        mfa,
        providers,
        domains,
        regions,
        audit_events,
        Utc::now(),
    ))
}

fn build_trust_center_response(
    tenant: db::TrustTenantRow,
    mfa: db::TrustMfaRow,
    providers: Vec<db::TrustSsoProviderRow>,
    domains: Vec<db::TrustDomainRow>,
    regions: Vec<db::TrustHostingRegionRow>,
    audit_events: Vec<EnterpriseAuditEvent>,
    generated_at: DateTime<Utc>,
) -> EnterpriseTrustCenterResponse {
    let active_providers = providers
        .iter()
        .filter(|provider| provider.status == "active")
        .count() as i64;
    let required_domains = domains.iter().filter(|domain| domain.sso_required).count() as i64;

    EnterpriseTrustCenterResponse {
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
                sso_provider_id: domain.sso_provider_id,
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
        generated_at,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_center_response_derives_statuses_from_tenant_rows() {
        let now = Utc::now();
        let tenant_id = Uuid::new_v4();

        let response = build_trust_center_response(
            db::TrustTenantRow {
                id: tenant_id,
                name: "Acme".to_string(),
                slug: "acme".to_string(),
                status: "active".to_string(),
                security_tier: "enterprise".to_string(),
            },
            db::TrustMfaRow {
                active_members: 3,
                members_with_mfa: 2,
                active_factors: 4,
                passkey_factors: 1,
            },
            vec![db::TrustSsoProviderRow {
                id: Uuid::new_v4(),
                name: "Okta".to_string(),
                provider_type: "saml".to_string(),
                provider_family: "okta".to_string(),
                status: "active".to_string(),
                created_at: now,
            }],
            vec![
                db::TrustDomainRow {
                    id: Uuid::new_v4(),
                    domain: "acme.com".to_string(),
                    verified_at: Some(now),
                    sso_required: true,
                    sso_provider_id: Some(Uuid::new_v4()),
                },
                db::TrustDomainRow {
                    id: Uuid::new_v4(),
                    domain: "pending.acme.com".to_string(),
                    verified_at: None,
                    sso_required: false,
                    sso_provider_id: None,
                },
            ],
            vec![db::TrustHostingRegionRow {
                data_region: "eu".to_string(),
                legal_jurisdiction: "gdpr".to_string(),
                workspace_count: 2,
            }],
            Vec::new(),
            now,
        );

        assert_eq!(response.tenant.id, tenant_id);
        assert!(response.mfa.enabled);
        assert_eq!(response.mfa.members_with_mfa, 2);
        assert!(response.sso.enabled);
        assert_eq!(response.sso.active_providers, 1);
        assert_eq!(response.sso.required_domains, 1);
        assert!(response.verified_domains[0].verified);
        assert!(!response.verified_domains[1].verified);
        assert!(response.audit.immutable);
        assert_eq!(response.hosting_regions[0].legal_jurisdiction, "gdpr");
        assert_eq!(response.dpa.version, "2026-05-11");
        assert!(
            response
                .subprocessors
                .iter()
                .any(|item| item.name == "Scaleway")
        );
    }
}
