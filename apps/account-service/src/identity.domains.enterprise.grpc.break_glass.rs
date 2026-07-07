use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domains::enterprise::types::EnterpriseBreakGlassAccount,
    grpc_pb::nvbes::enterprise::v1::{
        ActivateBreakGlassRequest, ListBreakGlassAccountsRequest, RevokeBreakGlassRequest,
    },
    http::error::AppError,
};

pub struct ActiveBreakGlassAccount {
    pub principal_id: Uuid,
    pub account: EnterpriseBreakGlassAccount,
}

pub async fn activate_break_glass(
    tenant_id: Uuid,
    principal_id: Uuid,
    actor_principal_id: Uuid,
    procedure_reference: &str,
    reason: &str,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .activate_break_glass(ActivateBreakGlassRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            principal_id: principal_id.to_string(),
            role: procedure_reference.to_string(),
            reason: reason.to_string(),
            expires_at: String::new(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

pub async fn revoke_break_glass(
    tenant_id: Uuid,
    principal_id: Uuid,
    actor_principal_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    let mut client = super::enterprise_client().await?;
    client
        .revoke_break_glass(RevokeBreakGlassRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            grant_id: principal_id.to_string(),
            reason: reason.to_string(),
        })
        .await
        .map_err(super::grpc_error)?;
    Ok(())
}

pub async fn list_active_break_glass_accounts(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    principal_ids: Vec<Uuid>,
) -> Result<Vec<ActiveBreakGlassAccount>, AppError> {
    let mut client = super::enterprise_client().await?;
    let response = client
        .list_break_glass_accounts(ListBreakGlassAccountsRequest {
            context: Some(super::request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            principal_ids: principal_ids
                .into_iter()
                .map(|principal_id| principal_id.to_string())
                .collect(),
        })
        .await
        .map_err(super::grpc_error)?
        .into_inner();
    response
        .accounts
        .into_iter()
        .map(account_from_grpc)
        .collect()
}

fn account_from_grpc(
    value: crate::grpc_pb::nvbes::enterprise::v1::BreakGlassAccount,
) -> Result<ActiveBreakGlassAccount, AppError> {
    Ok(ActiveBreakGlassAccount {
        principal_id: parse_uuid(&value.principal_id, "principal_id")?,
        account: EnterpriseBreakGlassAccount {
            procedure_reference: value.procedure_reference,
            reason: value.reason,
            created_at: parse_time(&value.created_at, "created_at")?,
            last_used_at: optional_time(&value.last_used_at, "last_used_at")?,
        },
    })
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_break_glass_account",
            format!("Enterprise gRPC returned invalid {field}: {error}"),
        )
    })
}

fn parse_time(value: &str, field: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_break_glass_account",
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
