use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::region_center_types::{RegionActionResult, action_result};
use crate::region_center_validation::{
    validate_data_region, validate_exception_kind, validate_jurisdiction, validate_reason,
};

pub(crate) struct RegionFlagInput {
    pub(crate) target_workspace_id: Uuid,
    pub(crate) data_region: String,
    pub(crate) jurisdiction: String,
    pub(crate) reason: String,
}

pub(crate) async fn flag_residency(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: RegionFlagInput,
) -> Result<RegionActionResult, AppError> {
    validate_data_region(&input.data_region)?;
    validate_jurisdiction(&input.jurisdiction)?;
    validate_reason(&input.reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, data_region::text AS previous_data_region,
             jurisdiction::text AS previous_jurisdiction
           FROM workspaces
           WHERE id = $3 AND tenant_id = $4
         ),
         updated AS (
           UPDATE workspaces w
           SET data_region = $1::data_region,
             jurisdiction = $2::legal_jurisdiction,
             updated_at = NOW()
           FROM previous
           WHERE w.id = previous.id
           RETURNING w.data_region::text AS next_data_region,
             w.jurisdiction::text AS next_jurisdiction
         )
         SELECT previous.previous_data_region, previous.previous_jurisdiction,
           updated.next_data_region, updated.next_jurisdiction
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(&input.data_region)
    .bind(&input.jurisdiction)
    .bind(input.target_workspace_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RegionActionInput {
            action_kind: "flag_residency",
            target_workspace_id: input.target_workspace_id,
            previous_data_region: Some(row.get("previous_data_region")),
            next_data_region: Some(row.get("next_data_region")),
            previous_jurisdiction: Some(row.get("previous_jurisdiction")),
            next_jurisdiction: Some(row.get("next_jurisdiction")),
            exception_kind: None,
            status: "applied",
            reason: input.reason,
            metadata: json!({}),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "region.residency.flagged",
        input.target_workspace_id,
        action_id,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "flag_residency",
        "applied",
        "region.residency.flagged",
    ))
}

pub(crate) async fn record_exception(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    target_workspace_id: Uuid,
    exception_kind: String,
    reason: String,
) -> Result<RegionActionResult, AppError> {
    validate_exception_kind(&exception_kind)?;
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = $1 AND tenant_id = $2)",
    )
    .bind(target_workspace_id)
    .bind(access.tenant_id)
    .fetch_one(tx.as_mut())
    .await?;
    if !exists {
        return Err(AppError::not_found(
            "workspace_not_found",
            "Workspace not found.",
        ));
    }
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RegionActionInput {
            action_kind: "record_exception",
            target_workspace_id,
            previous_data_region: None,
            next_data_region: None,
            previous_jurisdiction: None,
            next_jurisdiction: None,
            exception_kind: Some(exception_kind),
            status: "recorded",
            reason,
            metadata: json!({}),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        "region.residency_exception.recorded",
        target_workspace_id,
        action_id,
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "record_exception",
        "recorded",
        "region.residency_exception.recorded",
    ))
}

struct RegionActionInput {
    action_kind: &'static str,
    target_workspace_id: Uuid,
    previous_data_region: Option<String>,
    next_data_region: Option<String>,
    previous_jurisdiction: Option<String>,
    next_jurisdiction: Option<String>,
    exception_kind: Option<String>,
    status: &'static str,
    reason: String,
    metadata: Value,
}

async fn insert_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: RegionActionInput,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO internal_admin_region_actions (
           tenant_id, workspace_id, target_workspace_id, actor_principal_id, action_kind,
           previous_data_region, next_data_region, previous_jurisdiction, next_jurisdiction,
           exception_kind, status, reason, metadata
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(input.target_workspace_id)
    .bind(access.actor_principal_id)
    .bind(input.action_kind)
    .bind(input.previous_data_region)
    .bind(input.next_data_region)
    .bind(input.previous_jurisdiction)
    .bind(input.next_jurisdiction)
    .bind(input.exception_kind)
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
    target_workspace_id: Uuid,
    action_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, 'workspace', $2, jsonb_build_object('region_action_id', $5), gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(target_workspace_id)
    .bind(access.actor_principal_id)
    .bind(action)
    .bind(action_id)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}
