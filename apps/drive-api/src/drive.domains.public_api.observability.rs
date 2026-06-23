use crate::http::error::AppError;
use sqlx::PgPool;

use super::db;
use super::types::*;

pub async fn log_request(
    db: &PgPool,
    context: &PublicApiContext,
    input: PublicApiLogInput<'_>,
) -> Result<(), AppError> {
    db::insert_api_request_log(
        db,
        ApiRequestLogInsert {
            workspace_id: context.workspace_id,
            api_key_id: context.api_key_id,
            actor_principal_id: Some(context.created_by_principal_id),
            request_id: &context.request_id,
            method: input.method,
            path: input.path,
            status_code: input.status_code,
            error_code: input.error_code,
            scopes_used: input.scopes_used,
            ip: input.ip,
            user_agent: input.user_agent,
        },
    )
    .await
    .map_err(AppError::from)
}

pub async fn record_api_audit_event(
    db: &PgPool,
    context: &PublicApiContext,
    input: PublicApiAuditEventInput<'_>,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;
    db::insert_audit_event_tx(
        &mut tx,
        AuditEventInsert {
            workspace_id: context.workspace_id,
            actor_user_id: context.created_by,
            actor_principal_id: Some(context.created_by_principal_id),
            action: input.action,
            target_type: input.target_type,
            target_id: input.target_id,
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: serde_json::json!({
                "api_key_id": context.api_key_id,
                "key_prefix": context.key_prefix,
                "request_id": context.request_id,
                "details": input.metadata
            }),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn log_denied(db: &PgPool, input: DeniedLogInput<'_>) -> Result<(), AppError> {
    db::insert_api_request_log(
        db,
        ApiRequestLogInsert {
            workspace_id: input.workspace_id,
            api_key_id: input.api_key_id,
            actor_principal_id: input.actor_principal_id,
            request_id: input.request_id,
            method: "UNKNOWN",
            path: "/v1",
            status_code: 403,
            error_code: Some(input.error_code),
            scopes_used: input.scopes_used,
            ip: input.ip,
            user_agent: input.user_agent,
        },
    )
    .await?;

    let mut tx = db.begin().await?;
    db::insert_audit_event_tx(
        &mut tx,
        AuditEventInsert {
            workspace_id: input.workspace_id,
            actor_user_id: None,
            actor_principal_id: None,
            action: "api.request.denied",
            target_type: "api_key",
            target_id: input.api_key_id,
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: serde_json::json!({
                "request_id": input.request_id,
                "error_code": input.error_code,
                "network_block_reason": input.network_block_reason,
                "required_scopes": input.scopes_used
            }),
        },
    )
    .await?;
    tx.commit().await?;

    Ok(())
}
