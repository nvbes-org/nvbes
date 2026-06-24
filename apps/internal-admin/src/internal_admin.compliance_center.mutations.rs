use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::compliance_center_types::{ComplianceActionResult, action_result};
use crate::compliance_center_validation::{validate_email, validate_reason};
use crate::error::AppError;

pub(crate) async fn revoke_consent(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    consent_id: Uuid,
    reason: String,
) -> Result<ComplianceActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "UPDATE user_consents uc
         SET revoked_at = NOW()
         FROM principals p
         WHERE uc.id = $1 AND uc.principal_id = p.id
           AND p.tenant_id = $2 AND uc.revoked_at IS NULL
         RETURNING uc.id, uc.principal_id",
    )
    .bind(consent_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "consent_not_revokable",
            "Consent is missing, belongs to another tenant, or is already revoked.",
        )
    })?;
    let principal_id: Uuid = row.get("principal_id");
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        ComplianceActionInput {
            action_kind: "revoke_consent",
            principal_id: Some(principal_id),
            consent_id: Some(consent_id),
            email: None,
            status: "applied",
            reason,
            metadata: json!({ "consent_id": consent_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "compliance.consent.revoked",
        "user_consent",
        consent_id,
        action_id,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "revoke_consent",
        "applied",
        "compliance.consent.revoked",
    ))
}

pub(crate) async fn request_erasure(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    principal_id: Uuid,
    reason: String,
) -> Result<ComplianceActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "UPDATE principals
         SET status = 'deleted'
         WHERE id = $1 AND tenant_id = $2 AND status::text <> 'deleted'
         RETURNING id",
    )
    .bind(principal_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "principal_not_erasable",
            "Principal is missing, belongs to another tenant, or is already deleted.",
        )
    })?;
    let target_id: Uuid = row.get("id");
    sqlx::query("UPDATE users SET status = 'deleted', updated_at = NOW() WHERE principal_id = $1")
        .bind(principal_id)
        .execute(tx.as_mut())
        .await?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        ComplianceActionInput {
            action_kind: "request_erasure",
            principal_id: Some(target_id),
            consent_id: None,
            email: None,
            status: "requested",
            reason,
            metadata: json!({ "principal_id": target_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "compliance.erasure.requested",
        "principal",
        target_id,
        action_id,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "request_erasure",
        "requested",
        "compliance.erasure.requested",
    ))
}

pub(crate) async fn review_suppression(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    email: String,
    reason: String,
) -> Result<ComplianceActionResult, AppError> {
    validate_email(&email)?;
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let reviewed = sqlx::query(
        "UPDATE suppressed_emails
         SET details = COALESCE(details, '{}'::jsonb) || jsonb_build_object(
           'reviewed_by', $1::text,
           'review_reason', $2,
           'reviewed_at', NOW()
         )
         WHERE lower(email) = lower($3)",
    )
    .bind(access.actor_principal_id)
    .bind(&reason)
    .bind(&email)
    .execute(tx.as_mut())
    .await?
    .rows_affected();
    if reviewed == 0 {
        return Err(AppError::not_found(
            "suppressed_email_not_found",
            "Suppressed email not found.",
        ));
    }
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        ComplianceActionInput {
            action_kind: "review_suppression",
            principal_id: None,
            consent_id: None,
            email: Some(email),
            status: "reviewed",
            reason,
            metadata: json!({}),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "compliance.suppression.reviewed",
        "suppressed_email",
        action_id,
        action_id,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "review_suppression",
        "reviewed",
        "compliance.suppression.reviewed",
    ))
}

struct ComplianceActionInput {
    action_kind: &'static str,
    principal_id: Option<Uuid>,
    consent_id: Option<Uuid>,
    email: Option<String>,
    status: &'static str,
    reason: String,
    metadata: Value,
}

async fn insert_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: ComplianceActionInput,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO internal_admin_compliance_actions (
           tenant_id, workspace_id, actor_principal_id, action_kind, principal_id,
           consent_id, email, status, reason, metadata
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(input.action_kind)
    .bind(input.principal_id)
    .bind(input.consent_id)
    .bind(input.email)
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
           $1, $2, $3, $4, $5, jsonb_build_object('compliance_action_id', $6), gen_random_uuid()::text
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
