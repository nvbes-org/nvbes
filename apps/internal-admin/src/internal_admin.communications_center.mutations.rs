use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::communications_center_types::{CommunicationsActionResult, action_result};
use crate::communications_center_validation::{validate_email, validate_reason};
use crate::error::AppError;

pub(crate) async fn replay_email(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    message_id: Uuid,
    reason: String,
) -> Result<CommunicationsActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let email = sqlx::query_scalar::<_, String>(
        "UPDATE email_messages
         SET status = 'queued', updated_at = NOW()
         WHERE id = $1
         RETURNING recipient_email",
    )
    .bind(message_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| AppError::not_found("email_message_not_found", "Email message not found."))?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        CommunicationsActionInput {
            action_kind: "replay_email",
            email: Some(email),
            email_message_id: Some(message_id),
            email_event_id: None,
            status: "queued",
            reason,
            metadata: json!({ "message_id": message_id }),
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

pub(crate) async fn replay_webhook(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    event_id: Uuid,
    reason: String,
) -> Result<CommunicationsActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let email = sqlx::query_scalar::<_, String>(
        "UPDATE email_events
         SET processed_at = NULL
         WHERE id = $1
         RETURNING email",
    )
    .bind(event_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| AppError::not_found("email_event_not_found", "Email event not found."))?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        CommunicationsActionInput {
            action_kind: "replay_webhook",
            email: Some(email),
            email_message_id: None,
            email_event_id: Some(event_id),
            status: "queued",
            reason,
            metadata: json!({ "event_id": event_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "communications.webhook.replayed",
        "email_event",
        event_id,
        action_id,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "replay_webhook",
        "queued",
        "communications.webhook.replayed",
    ))
}

pub(crate) async fn suppress_email(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    email: String,
    reason: String,
) -> Result<CommunicationsActionResult, AppError> {
    validate_email(&email)?;
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO suppressed_emails (email, reason, details)
         VALUES ($1, $2, jsonb_build_object('source', 'internal_admin'))
         ON CONFLICT (email) DO UPDATE
         SET reason = EXCLUDED.reason, suppressed_at = NOW(), details = EXCLUDED.details",
    )
    .bind(&email)
    .bind(&reason)
    .execute(tx.as_mut())
    .await?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        CommunicationsActionInput::email("suppress_email", email, "applied", reason),
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "communications.email.suppressed",
        "suppressed_email",
        action_id,
        action_id,
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
    access: BackofficeAccess,
    workspace_id: Uuid,
    email: String,
    reason: String,
) -> Result<CommunicationsActionResult, AppError> {
    validate_email(&email)?;
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let removed = sqlx::query("DELETE FROM suppressed_emails WHERE email = $1")
        .bind(&email)
        .execute(tx.as_mut())
        .await?
        .rows_affected();
    if removed == 0 {
        return Err(AppError::not_found(
            "suppressed_email_not_found",
            "Suppressed email not found.",
        ));
    }
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        CommunicationsActionInput::email("unsuppress_email", email, "removed", reason),
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "communications.email.unsuppressed",
        "suppressed_email",
        action_id,
        action_id,
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

impl CommunicationsActionInput {
    fn email(
        action_kind: &'static str,
        email: String,
        status: &'static str,
        reason: String,
    ) -> Self {
        Self {
            action_kind,
            email: Some(email),
            email_message_id: None,
            email_event_id: None,
            status,
            reason,
            metadata: json!({}),
        }
    }
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
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5, jsonb_build_object('communications_action_id', $6), gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(action_id)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}
