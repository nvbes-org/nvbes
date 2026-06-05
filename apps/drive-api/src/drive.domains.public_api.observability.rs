use crate::http::error::AppError;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

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
    insert_audit_event_pool(
        db,
        context.workspace_id,
        context.created_by,
        Some(context.created_by_principal_id),
        input.action,
        input.target_type,
        input.target_id,
        input.ip,
        input.user_agent,
        serde_json::json!({
            "api_key_id": context.api_key_id,
            "key_prefix": context.key_prefix,
            "request_id": context.request_id,
            "details": input.metadata
        }),
    )
    .await
    .map_err(AppError::from)
}

async fn insert_audit_event_pool(
    pool: &sqlx::Pool<Postgres>,
    workspace_id: Uuid,
    actor_user_id: Option<Uuid>,
    actor_principal_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    ip: Option<&str>,
    user_agent: Option<&str>,
    metadata: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_user_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6::inet, $7, $8)
        "#,
    )
    .bind(workspace_id)
    .bind(actor_user_id)
    .bind(actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(ip)
    .bind(user_agent)
    .bind(sqlx::types::Json(metadata))
    .execute(pool)
    .await?;

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
                "required_scopes": input.scopes_used
            }),
        },
    )
    .await?;
    tx.commit().await?;

    Ok(())
}
