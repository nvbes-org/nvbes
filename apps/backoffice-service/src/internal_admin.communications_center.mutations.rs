use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::communications_center_types::{CommunicationsActionResult, action_result};
use crate::communications_center_validation::{validate_email, validate_reason};
use crate::error::AppError;

pub(crate) async fn replay_email(
    db: &PgPool,
    email_operations: &dyn crate::email_operations::BackofficeEmailOperations,
    access: BackofficeAccess,
    workspace_id: Uuid,
    message_id: Uuid,
    reason: String,
) -> Result<CommunicationsActionResult, AppError> {
    validate_reason(&reason)?;
    let receipt = email_operations
        .replay_email(access, workspace_id, message_id, &reason)
        .await?;
    let mut tx = db.begin().await?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        CommunicationsActionInput {
            action_kind: "replay_email",
            email: None,
            email_message_id: Some(message_id),
            email_event_id: None,
            status: "queued",
            reason,
            metadata: json!({
                "message_id": message_id,
                "email_operator_action_id": receipt.action_id,
            }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "communications.email.replayed",
        "email_message",
        message_id,
        action_id,
        json!({
            "object_links": {
                "email_message_id": message_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": "recorded",
                    "after": "queued",
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "replay_email",
        "queued",
        "communications.email.replayed",
    ))
}

pub(crate) async fn suppress_email(
    db: &PgPool,
    email_operations: &dyn crate::email_operations::BackofficeEmailOperations,
    access: BackofficeAccess,
    workspace_id: Uuid,
    email: String,
    reason: String,
) -> Result<CommunicationsActionResult, AppError> {
    validate_email(&email)?;
    validate_reason(&reason)?;
    let receipt = email_operations
        .apply_suppression(access, workspace_id, &email, &reason)
        .await?;
    let mut tx = db.begin().await?;
    let redacted_email = redact_email(&email);
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        CommunicationsActionInput {
            action_kind: "suppress_email",
            email: Some(email),
            email_message_id: None,
            email_event_id: None,
            status: "applied",
            reason,
            metadata: json!({ "email_operator_action_id": receipt.action_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "communications.email.suppressed",
        "suppressed_email",
        action_id,
        action_id,
        json!({
            "object_links": {
                "communications_action_id": action_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "suppression",
                    "before": null,
                    "after": "applied",
                },
                {
                    "field": "email",
                    "before": null,
                    "after": redacted_email,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "suppress_email",
        "applied",
        "communications.email.suppressed",
    ))
}

pub(crate) async fn unsuppress_email(
    db: &PgPool,
    email_operations: &dyn crate::email_operations::BackofficeEmailOperations,
    access: BackofficeAccess,
    workspace_id: Uuid,
    email: String,
    reason: String,
) -> Result<CommunicationsActionResult, AppError> {
    validate_email(&email)?;
    validate_reason(&reason)?;
    let receipt = email_operations
        .release_suppression(access, workspace_id, &email, &reason)
        .await?;
    let mut tx = db.begin().await?;
    let redacted_email = redact_email(&email);
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        CommunicationsActionInput {
            action_kind: "unsuppress_email",
            email: Some(email),
            email_message_id: None,
            email_event_id: None,
            status: "removed",
            reason,
            metadata: json!({ "email_operator_action_id": receipt.action_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "communications.email.unsuppressed",
        "suppressed_email",
        action_id,
        action_id,
        json!({
            "object_links": {
                "communications_action_id": action_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "suppression",
                    "before": "applied",
                    "after": null,
                },
                {
                    "field": "email",
                    "before": redacted_email,
                    "after": null,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "unsuppress_email",
        "removed",
        "communications.email.unsuppressed",
    ))
}

struct CommunicationsActionInput {
    action_kind: &'static str,
    email: Option<String>,
    email_message_id: Option<Uuid>,
    email_event_id: Option<Uuid>,
    status: &'static str,
    reason: String,
    metadata: Value,
}

async fn insert_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: CommunicationsActionInput,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO internal_admin_communications_actions (
           tenant_id, workspace_id, actor_principal_id, action_kind, email,
           email_message_id, email_event_id, status, reason, metadata
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(input.action_kind)
    .bind(input.email)
    .bind(input.email_message_id)
    .bind(input.email_event_id)
    .bind(input.status)
    .bind(input.reason)
    .bind(input.metadata)
    .fetch_one(tx.as_mut())
    .await
    .map_err(AppError::from)
}

async fn insert_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    action: &'static str,
    target_type: &'static str,
    target_id: Uuid,
    action_id: Uuid,
    metadata: Value,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5,
           jsonb_build_object('communications_action_id', $6) || $7::jsonb,
           gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(action_id)
    .bind(metadata)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

fn redact_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return "redacted".to_string();
    };
    let first = local.chars().next().unwrap_or('*');
    format!("{first}***@{domain}")
}
