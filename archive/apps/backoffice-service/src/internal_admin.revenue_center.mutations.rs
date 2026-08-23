use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::billing_grpc_revenue::{
    BackofficeRevenueActionKind, BackofficeRevenueActionOutcome, run_revenue_grpc_action,
};
use crate::error::AppError;
use crate::revenue_center_action_log::{RevenueActionInput, insert_action, insert_audit};
use crate::revenue_center_action_values::{action_kind, action_status, audit_action, target_type};
use crate::revenue_center_types::{RevenueActionResult, action_result};
use crate::revenue_center_validation::validate_reason;

pub(crate) async fn close_dunning_case(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    case_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    run_revenue_action(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        RevenueTarget::DunningCase(case_id),
        BackofficeRevenueActionKind::CloseDunningCase,
        reason,
    )
    .await
}

pub(crate) async fn reopen_dunning_case(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    case_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    run_revenue_action(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        RevenueTarget::DunningCase(case_id),
        BackofficeRevenueActionKind::ReopenDunningCase,
        reason,
    )
    .await
}

pub(crate) async fn hold_invoice(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    invoice_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    run_revenue_action(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        RevenueTarget::Invoice(invoice_id),
        BackofficeRevenueActionKind::HoldInvoice,
        reason,
    )
    .await
}

pub(crate) async fn release_invoice(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    invoice_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    run_revenue_action(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        RevenueTarget::Invoice(invoice_id),
        BackofficeRevenueActionKind::ReleaseInvoice,
        reason,
    )
    .await
}

pub(crate) async fn review_dispute(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    dispute_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    run_revenue_action(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        RevenueTarget::Dispute(dispute_id),
        BackofficeRevenueActionKind::ReviewDispute,
        reason,
    )
    .await
}

pub(crate) async fn resolve_dispute(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    dispute_id: Uuid,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    run_revenue_action(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        RevenueTarget::Dispute(dispute_id),
        BackofficeRevenueActionKind::ResolveDispute,
        reason,
    )
    .await
}

enum RevenueTarget {
    DunningCase(Uuid),
    Invoice(Uuid),
    Dispute(Uuid),
}

async fn run_revenue_action(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    target: RevenueTarget,
    action_kind: BackofficeRevenueActionKind,
    reason: String,
) -> Result<RevenueActionResult, AppError> {
    validate_reason(&reason)?;
    let target_id = target_id(&target);
    let outcome = run_revenue_grpc_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        action_kind,
        target_id,
        reason.clone(),
    )
    .await?;
    record_revenue_action(db, access, workspace_id, reason, target, outcome).await
}

async fn record_revenue_action(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    reason: String,
    target: RevenueTarget,
    outcome: BackofficeRevenueActionOutcome,
) -> Result<RevenueActionResult, AppError> {
    let action_kind = action_kind(&outcome.action_kind)?;
    let next_state = action_status(&outcome.status)?;
    let audit_action = audit_action(&outcome.audit_action)?;
    let target_type = target_type(&outcome.target_type)?;
    let audit_metadata = audit_metadata(workspace_id, &target, &outcome);
    let mut tx = db.begin().await?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        RevenueActionInput {
            action_kind,
            dunning_case_id: matches_target_id(&target, "dunning_case"),
            invoice_id: matches_target_id(&target, "invoice"),
            dispute_id: matches_target_id(&target, "dispute"),
            previous_state: outcome.previous_state.clone(),
            next_state,
            reason,
            metadata: outcome.metadata,
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        audit_action,
        target_type,
        outcome.object_id,
        action_id,
        audit_metadata,
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

fn target_id(target: &RevenueTarget) -> Uuid {
    match target {
        RevenueTarget::DunningCase(id)
        | RevenueTarget::Invoice(id)
        | RevenueTarget::Dispute(id) => *id,
    }
}

fn matches_target_id(target: &RevenueTarget, field: &str) -> Option<Uuid> {
    match (target, field) {
        (RevenueTarget::DunningCase(id), "dunning_case") => Some(*id),
        (RevenueTarget::Invoice(id), "invoice") => Some(*id),
        (RevenueTarget::Dispute(id), "dispute") => Some(*id),
        _ => None,
    }
}

fn audit_metadata(
    workspace_id: Uuid,
    target: &RevenueTarget,
    outcome: &BackofficeRevenueActionOutcome,
) -> Value {
    let previous_state = outcome.previous_state.as_deref();
    match target {
        RevenueTarget::DunningCase(id) => json!({
            "object_links": {
                "dunning_case_id": id,
                "workspace_id": workspace_id,
            },
            "changes": [{
                "field": "status",
                "before": previous_state,
                "after": outcome.status,
            }],
        }),
        RevenueTarget::Invoice(id) => json!({
            "object_links": {
                "invoice_id": id,
                "workspace_id": workspace_id,
            },
            "changes": [{
                "field": "internal_hold",
                "before": outcome.action_kind == "release_invoice",
                "after": outcome.action_kind == "hold_invoice",
            }],
        }),
        RevenueTarget::Dispute(id) => json!({
            "object_links": {
                "dispute_id": id,
                "workspace_id": workspace_id,
            },
            "changes": [{
                "field": "status",
                "before": previous_state,
                "after": outcome.status,
            }],
        }),
    }
}
