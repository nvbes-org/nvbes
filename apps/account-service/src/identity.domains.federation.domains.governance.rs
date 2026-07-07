use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domains::federation::types::TenantDomainView, grpc_pb::nvbes::enterprise::v1,
    http::error::AppError,
};

pub struct TenantDomain {
    pub domain: String,
    pub sso_required: bool,
    pub sso_provider_id: Option<Uuid>,
    pub verification_expires_at: Option<DateTime<Utc>>,
    pub verification_token_hash: Option<String>,
    id: Uuid,
    verified_at: Option<DateTime<Utc>>,
    verification_requested_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TenantDomain {
    pub fn into_view(self) -> TenantDomainView {
        TenantDomainView {
            id: self.id,
            domain: self.domain,
            sso_required: self.sso_required,
            sso_provider_id: self.sso_provider_id,
            verified_at: self.verified_at,
            verification_requested_at: self.verification_requested_at,
            verification_expires_at: self.verification_expires_at,
            created_at: self.created_at,
        }
    }
}

pub fn tenant_domain_from_grpc(domain: v1::TenantDomain) -> Result<TenantDomain, AppError> {
    Ok(TenantDomain {
        id: parse_uuid(&domain.domain_id, "domain_id")?,
        domain: domain.domain,
        sso_required: domain.sso_required,
        sso_provider_id: optional_uuid(&domain.sso_provider_id, "sso_provider_id")?,
        verified_at: optional_time(&domain.verified_at, "verified_at")?,
        verification_requested_at: optional_time(
            &domain.verification_requested_at,
            "verification_requested_at",
        )?,
        verification_expires_at: optional_time(
            &domain.verification_expires_at,
            "verification_expires_at",
        )?,
        verification_token_hash: optional_text(domain.verification_token_hash),
        created_at: parse_time(&domain.created_at, "created_at")?,
    })
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_federation_governance",
            format!("Enterprise gRPC returned invalid {field}: {error}"),
        )
    })
}

fn optional_uuid(value: &str, field: &str) -> Result<Option<Uuid>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn parse_time(value: &str, field: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_federation_governance",
                format!("Enterprise gRPC returned invalid {field}: {error}"),
            )
        })
}

fn optional_time(value: &str, field: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_time(value, field).map(Some)
    }
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
