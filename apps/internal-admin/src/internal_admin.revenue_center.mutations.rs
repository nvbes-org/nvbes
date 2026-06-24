use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;
use crate::revenue_center_types::{RevenueActionResult, action_result};
use crate::revenue_center_validation::validate_reason;

pub(crate) async fn close_dunning_case(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    case_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    transition_dunning_case(db, access, workspace_id, case_id, "closed", reason).await
}

pub(crate) async fn reopen_dunning_case(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    case_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    transition_dunning_case(db, access, workspace_id, case_id, "open", reason).await
}

pub(crate) async fn hold_invoice(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    invoice_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "UPDATE billing_invoices
         SET internal_hold_at = NOW(), internal_hold_reason = $1,
           internal_hold_by_principal_id = $2, updated_at = NOW()
         WHERE id = $3 AND tenant_id = $4 AND internal_hold_at IS NULL
           AND status::text IN ('draft', 'pro_forma', 'issued')
         RETURNING status::text AS previous_state",
    )
    .bind(&reason)
    .bind(access.actor_principal_id)
    .bind(invoice_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "invoice_not_holdable",
            "Invoice is missing, belongs to another tenant, already held, or not holdable.",
        )
    })?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RevenueActionInput {
            action_kind: "hold_invoice",
            dunning_case_id: None,
            invoice_id: Some(invoice_id),
            dispute_id: None,
            previous_state: Some(row.get("previous_state")),
            next_state: "held",
            reason,
            metadata: json!({ "invoice_id": invoice_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "revenue.invoice.held",
        "billing_invoice",
        invoice_id,
        action_id,
        json!({
            "object_links": {
                "invoice_id": invoice_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "internal_hold",
                    "before": false,
                    "after": true,
                },
                {
                    "field": "internal_hold_reason",
                    "before": null,
                    "after": "recorded",
                },
                {
                    "field": "internal_hold_by_principal_id",
                    "before": null,
                    "after": access.actor_principal_id,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "hold_invoice",
        "held",
        "revenue.invoice.held",
    ))
}

pub(crate) async fn release_invoice(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    invoice_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "UPDATE billing_invoices
         SET internal_hold_at = NULL, internal_hold_reason = NULL,
           internal_hold_by_principal_id = NULL, updated_at = NOW()
         WHERE id = $1 AND tenant_id = $2 AND internal_hold_at IS NOT NULL
         RETURNING status::text AS next_state",
    )
    .bind(invoice_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "invoice_not_releasable",
            "Invoice is missing, belongs to another tenant, or is not held.",
        )
    })?;
    let next_state: String = row.get("next_state");
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RevenueActionInput {
            action_kind: "release_invoice",
            dunning_case_id: None,
            invoice_id: Some(invoice_id),
            dispute_id: None,
            previous_state: Some("held".to_string()),
            next_state: "released",
            reason,
            metadata: json!({ "invoice_status": next_state }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "revenue.invoice.released",
        "billing_invoice",
        invoice_id,
        action_id,
        json!({
            "object_links": {
                "invoice_id": invoice_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "internal_hold",
                    "before": true,
                    "after": false,
                },
                {
                    "field": "internal_hold_reason",
                    "before": "recorded",
                    "after": null,
                },
                {
                    "field": "internal_hold_by_principal_id",
                    "before": "recorded",
                    "after": null,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "release_invoice",
        "released",
        "revenue.invoice.released",
    ))
}

pub(crate) async fn review_dispute(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    dispute_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    transition_dispute(db, access, workspace_id, dispute_id, "under_review", reason).await
}

pub(crate) async fn resolve_dispute(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    dispute_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    transition_dispute(db, access, workspace_id, dispute_id, "resolved", reason).await
}

async fn transition_dunning_case(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    case_id: Uuid,
    next_state: &'static str,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, status AS previous_state
           FROM billing_dunning_cases
           WHERE id = $2 AND tenant_id = $3 AND status <> $1
         ),
         updated AS (
           UPDATE billing_dunning_cases bdc
           SET status = $1, closed_at = CASE WHEN $1 = 'closed' THEN NOW() ELSE NULL END,
             updated_at = NOW()
           FROM previous
           WHERE bdc.id = previous.id
           RETURNING bdc.status AS next_state
         )
         SELECT previous.previous_state, updated.next_state
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(next_state)
    .bind(case_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "dunning_case_not_transitionable",
            "Dunning case is missing, belongs to another tenant, or already has the requested status.",
        )
    })?;
    let (action_kind, audit_action) = if next_state == "closed" {
        ("close_dunning_case", "revenue.dunning_case.closed")
    } else {
        ("reopen_dunning_case", "revenue.dunning_case.reopened")
    };
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RevenueActionInput {
            action_kind,
            dunning_case_id: Some(case_id),
            invoice_id: None,
            dispute_id: None,
            previous_state: Some(row.get("previous_state")),
            next_state,
            reason,
            metadata: json!({ "dunning_case_id": case_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        audit_action,
        "billing_dunning_case",
        case_id,
        action_id,
        json!({
            "object_links": {
                "dunning_case_id": case_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": row.get::<String, _>("previous_state"),
                    "after": next_state,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        action_kind,
        next_state,
        audit_action,
    ))
}

async fn transition_dispute(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    dispute_id: Uuid,
    next_state: &'static str,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, status AS previous_state
           FROM billing_disputes
           WHERE id = $2 AND tenant_id = $3 AND status <> $1
         ),
         updated AS (
           UPDATE billing_disputes bd
           SET status = $1, updated_at = NOW()
           FROM previous
           WHERE bd.id = previous.id
           RETURNING bd.status AS next_state
         )
         SELECT previous.previous_state, updated.next_state
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(next_state)
    .bind(dispute_id)
    .bind(access.tenant_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "dispute_not_transitionable",
            "Dispute is missing, belongs to another tenant, or already has the requested status.",
        )
    })?;
    let (action_kind, audit_action) = if next_state == "resolved" {
        ("resolve_dispute", "revenue.dispute.resolved")
    } else {
        ("review_dispute", "revenue.dispute.reviewed")
    };
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RevenueActionInput {
            action_kind,
            dunning_case_id: None,
            invoice_id: None,
            dispute_id: Some(dispute_id),
            previous_state: Some(row.get("previous_state")),
            next_state,
            reason,
            metadata: json!({ "dispute_id": dispute_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        audit_action,
        "billing_dispute",
        dispute_id,
        action_id,
        json!({
            "object_links": {
                "dispute_id": dispute_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": row.get::<String, _>("previous_state"),
                    "after": next_state,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        action_kind,
        next_state,
        audit_action,
    ))
}

struct RevenueActionInput {
    action_kind: &'static str,
    dunning_case_id: Option<Uuid>,
    invoice_id: Option<Uuid>,
    dispute_id: Option<Uuid>,
    previous_state: Option<String>,
    next_state: &'static str,
    reason: String,
    metadata: Value,
}

async fn insert_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: RevenueActionInput,
) -> Result<Uuid, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO internal_admin_revenue_actions (
           tenant_id, workspace_id, actor_principal_id, action_kind, dunning_case_id,
           invoice_id, dispute_id, previous_state, next_state, reason, metadata
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING id",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
    .bind(access.actor_principal_id)
    .bind(input.action_kind)
    .bind(input.dunning_case_id)
    .bind(input.invoice_id)
    .bind(input.dispute_id)
    .bind(input.previous_state)
    .bind(input.next_state)
    .bind(input.reason)
    .bind(input.metadata)
    .fetch_one(tx.as_mut())
    .await
    .map_err(AppError::from)
}

async fn insert_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    access: BackofficeAccess,
    workspace_id: Uuid,
    action: &'static str,
    target_type: &'static str,
    target_id: Uuid,
    action_id: Uuid,
    metadata: Value,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5, $6,
           jsonb_build_object('revenue_action_id', $7) || $8::jsonb,
           gen_random_uuid()::text
         )",
    )
    .bind(access.tenant_id)
    .bind(workspace_id)
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
