use chrono::{DateTime, Utc};

use crate::{
    grpc::pb::nvbes::developer::v1::{HealthCheck, Sandbox},
    http::{
        context::{optional_time, time, uuid},
        error::AppError,
        types::{
            DeveloperHealthCheckSummary, DeveloperSandboxTenantSummary, DeveloperTokenClaimsView,
            InspectDeveloperTokenResponse,
        },
    },
};

use super::DebugTokenClaims;

pub(super) fn debug_claims_view(
    claims: DebugTokenClaims,
) -> Result<DeveloperTokenClaimsView, AppError> {
    Ok(DeveloperTokenClaimsView {
        subject: claims.sub,
        tenant_id: claims.tenant_id,
        workspace_id: claims.workspace_id,
        client_id: claims.client_id,
        scopes: split_scopes(Some(claims.scope)),
        audience: claims.aud,
        issuer: claims.iss,
        expires_at: timestamp(claims.exp)?,
        issued_at: timestamp(claims.iat)?,
        not_before: timestamp(claims.nbf)?,
        token_type: claims.token_type,
        amr: claims.amr,
        acr: claims.acr,
    })
}

pub(super) fn sandbox_view(sandbox: Sandbox) -> Result<DeveloperSandboxTenantSummary, AppError> {
    Ok(DeveloperSandboxTenantSummary {
        tenant_id: uuid(&sandbox.tenant_id, "tenant id")?,
        sandbox_tenant_id: uuid(&sandbox.sandbox_id, "sandbox id")?,
        sandbox_name: sandbox.sandbox_name,
        sandbox_slug: sandbox.sandbox_slug,
        status: sandbox.status,
        data_profile: sandbox.data_profile,
        reset_requested_at: optional_time(&sandbox.reset_at, "reset_at")?,
        updated_at: time(&sandbox.updated_at, "updated_at")?,
    })
}

pub(super) fn health_checks(
    checks: Vec<HealthCheck>,
) -> Result<Vec<DeveloperHealthCheckSummary>, AppError> {
    checks
        .into_iter()
        .map(|check| {
            Ok(DeveloperHealthCheckSummary {
                id: uuid(&check.id, "health check id")?,
                target_type: check.target_type,
                target_id: check.target_id,
                check_kind: check.check_kind,
                status: check.status,
                summary: check.summary,
                checked_at: time(&check.checked_at, "checked_at")?,
            })
        })
        .collect()
}

pub(super) fn inactive_token() -> InspectDeveloperTokenResponse {
    InspectDeveloperTokenResponse {
        active: false,
        subject: None,
        client_id: None,
        tenant_id: None,
        scopes: Vec::new(),
        expires_at: None,
    }
}

pub(super) fn split_scopes(scope: Option<String>) -> Vec<String> {
    scope
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

pub(super) fn json_string(
    value: &serde_json::Value,
    field: &'static str,
) -> Result<String, AppError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| invalid_identity_contract(field))
}

pub(super) fn invalid_identity_contract(field: &'static str) -> AppError {
    AppError::internal(
        "identity_contract_invalid",
        format!("Account token response is missing {field}."),
    )
}

fn timestamp(value: i64) -> Result<DateTime<Utc>, AppError> {
    DateTime::from_timestamp(value, 0).ok_or_else(|| {
        AppError::bad_request(
            "invalid_token_timestamp",
            "Token timestamp is out of range.",
        )
    })
}
