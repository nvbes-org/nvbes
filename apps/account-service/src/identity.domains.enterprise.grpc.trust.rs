use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::{
    domains::enterprise::{
        trust::types::{
            EnterpriseTrustCenterResponse, TrustCenterAuditStatus, TrustCenterDocument,
            TrustCenterDomain, TrustCenterHostingRegion, TrustCenterMfaStatus,
            TrustCenterSsoProvider, TrustCenterSsoStatus, TrustCenterSubprocessor,
            TrustCenterTenant,
        },
        types::EnterpriseAuditEvent,
    },
    grpc_pb::nvbes::enterprise::v1::{
        GetTrustCenterRequest, TrustCenter as GrpcTrustCenter,
        TrustCenterAuditEvent as GrpcTrustCenterAuditEvent,
        TrustCenterAuditStatus as GrpcTrustCenterAuditStatus,
        TrustCenterDocument as GrpcTrustCenterDocument, TrustCenterDomain as GrpcTrustCenterDomain,
        TrustCenterHostingRegion as GrpcTrustCenterHostingRegion,
        TrustCenterMfaStatus as GrpcTrustCenterMfaStatus,
        TrustCenterSsoProvider as GrpcTrustCenterSsoProvider,
        TrustCenterSsoStatus as GrpcTrustCenterSsoStatus,
        TrustCenterSubprocessor as GrpcTrustCenterSubprocessor,
        TrustCenterTenant as GrpcTrustCenterTenant,
    },
    http::error::AppError,
};

pub async fn get_trust_center(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<EnterpriseTrustCenterResponse, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .get_trust_center(GetTrustCenterRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(super::grpc_error)?
        .into_inner();
    trust_center_from_grpc(response)
}

fn trust_center_from_grpc(
    trust_center: GrpcTrustCenter,
) -> Result<EnterpriseTrustCenterResponse, AppError> {
    Ok(EnterpriseTrustCenterResponse {
        tenant: tenant_from_grpc(required(trust_center.tenant, "tenant")?)?,
        mfa: mfa_from_grpc(required(trust_center.mfa, "mfa")?),
        sso: sso_from_grpc(required(trust_center.sso, "sso")?)?,
        verified_domains: trust_center
            .verified_domains
            .into_iter()
            .map(domain_from_grpc)
            .collect::<Result<_, _>>()?,
        audit: audit_from_grpc(required(trust_center.audit, "audit")?)?,
        hosting_regions: trust_center
            .hosting_regions
            .into_iter()
            .map(hosting_region_from_grpc)
            .collect(),
        dpa: document_from_grpc(required(trust_center.dpa, "dpa")?),
        subprocessors: trust_center
            .subprocessors
            .into_iter()
            .map(subprocessor_from_grpc)
            .collect(),
        generated_at: parse_time(&trust_center.generated_at, "generated_at")?,
    })
}

fn tenant_from_grpc(value: GrpcTrustCenterTenant) -> Result<TrustCenterTenant, AppError> {
    Ok(TrustCenterTenant {
        id: parse_uuid(&value.tenant_id, "tenant_id")?,
        name: value.name,
        slug: value.slug,
        status: value.status,
        security_tier: value.security_tier,
    })
}

fn mfa_from_grpc(value: GrpcTrustCenterMfaStatus) -> TrustCenterMfaStatus {
    TrustCenterMfaStatus {
        enabled: value.enabled,
        active_members: value.active_members,
        members_with_mfa: value.members_with_mfa,
        active_factors: value.active_factors,
        passkey_factors: value.passkey_factors,
    }
}

fn sso_from_grpc(value: GrpcTrustCenterSsoStatus) -> Result<TrustCenterSsoStatus, AppError> {
    Ok(TrustCenterSsoStatus {
        enabled: value.enabled,
        active_providers: value.active_providers,
        required_domains: value.required_domains,
        providers: value
            .providers
            .into_iter()
            .map(provider_from_grpc)
            .collect::<Result<_, _>>()?,
    })
}

fn provider_from_grpc(
    value: GrpcTrustCenterSsoProvider,
) -> Result<TrustCenterSsoProvider, AppError> {
    Ok(TrustCenterSsoProvider {
        id: parse_uuid(&value.provider_id, "provider_id")?,
        name: value.name,
        provider_type: value.provider_type,
        provider_family: value.provider_family,
        status: value.status,
        created_at: parse_time(&value.created_at, "created_at")?,
    })
}

fn domain_from_grpc(value: GrpcTrustCenterDomain) -> Result<TrustCenterDomain, AppError> {
    Ok(TrustCenterDomain {
        id: parse_uuid(&value.domain_id, "domain_id")?,
        domain: value.domain,
        verified: value.verified,
        sso_required: value.sso_required,
        sso_provider_id: optional_uuid(&value.sso_provider_id, "sso_provider_id")?,
        verified_at: optional_time(&value.verified_at, "verified_at")?,
    })
}

fn audit_from_grpc(value: GrpcTrustCenterAuditStatus) -> Result<TrustCenterAuditStatus, AppError> {
    Ok(TrustCenterAuditStatus {
        immutable: value.immutable,
        recent_events: value
            .recent_events
            .into_iter()
            .map(audit_event_from_grpc)
            .collect::<Result<_, _>>()?,
    })
}

fn audit_event_from_grpc(
    value: GrpcTrustCenterAuditEvent,
) -> Result<EnterpriseAuditEvent, AppError> {
    Ok(EnterpriseAuditEvent {
        id: parse_uuid(&value.event_id, "event_id")?,
        event_type: value.event_type,
        actor_id: optional_uuid(&value.actor_id, "actor_id")?,
        actor_email: empty_to_option(value.actor_email),
        target_type: empty_to_option(value.target_type),
        target_id: optional_uuid(&value.target_id, "target_id")?,
        metadata: metadata(&value.metadata_json)?,
        created_at: parse_time(&value.created_at, "created_at")?,
    })
}

fn hosting_region_from_grpc(value: GrpcTrustCenterHostingRegion) -> TrustCenterHostingRegion {
    TrustCenterHostingRegion {
        data_region: value.data_region,
        legal_jurisdiction: value.legal_jurisdiction,
        workspace_count: value.workspace_count,
    }
}

fn document_from_grpc(value: GrpcTrustCenterDocument) -> TrustCenterDocument {
    TrustCenterDocument {
        name: value.name,
        status: value.status,
        version: value.version,
        url: value.url,
    }
}

fn subprocessor_from_grpc(value: GrpcTrustCenterSubprocessor) -> TrustCenterSubprocessor {
    TrustCenterSubprocessor {
        name: value.name,
        service: value.service,
        data_categories: value.data_categories,
        location: value.location,
        transfer_outside_eea: value.transfer_outside_eea,
        transfer_safeguard: value.transfer_safeguard,
    }
}

fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value.trim()).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_trust_center",
            format!("{field} from Enterprise gRPC is invalid: {error}"),
        )
    })
}

fn optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn parse_time(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|time| time.with_timezone(&Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_trust_center",
                format!("{field} from Enterprise gRPC is invalid: {error}"),
            )
        })
}

fn optional_time(value: &str, field: &'static str) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_time(value, field).map(Some)
    }
}

fn metadata(value: &str) -> Result<Option<BTreeMap<String, serde_json::Value>>, AppError> {
    match empty_to_option(value.to_string()) {
        Some(value) => serde_json::from_str(&value).map(Some).map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_trust_center",
                format!("metadata_json from Enterprise gRPC is invalid: {error}"),
            )
        }),
        None => Ok(None),
    }
}

fn required<T>(value: Option<T>, field: &'static str) -> Result<T, AppError> {
    value.ok_or_else(|| {
        AppError::internal(
            "enterprise_grpc_invalid_trust_center",
            format!("{field} from Enterprise gRPC is missing"),
        )
    })
}

fn empty_to_option(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
