use serde_json::json;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::billing::v1::{AdminRevenueActionKind, AdminRevenueActionResult};

pub async fn run_revenue_action(
    db: &sqlx::PgPool,
    kind: AdminRevenueActionKind,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    target_id: Uuid,
    reason: String,
) -> Result<AdminRevenueActionResult, Status> {
    validate_reason(&reason)?;
    match kind {
        AdminRevenueActionKind::CloseDunningCase => {
            transition_dunning_case(db, tenant_id, target_id, "closed").await
        }
        AdminRevenueActionKind::ReopenDunningCase => {
            transition_dunning_case(db, tenant_id, target_id, "open").await
        }
        AdminRevenueActionKind::HoldInvoice => {
            hold_invoice(db, tenant_id, actor_principal_id, target_id, reason).await
        }
        AdminRevenueActionKind::ReleaseInvoice => release_invoice(db, tenant_id, target_id).await,
        AdminRevenueActionKind::ReviewDispute => {
            transition_dispute(db, tenant_id, target_id, "under_review").await
        }
        AdminRevenueActionKind::ResolveDispute => {
            transition_dispute(db, tenant_id, target_id, "resolved").await
        }
        AdminRevenueActionKind::Unspecified => Err(Status::invalid_argument(
            "admin revenue action kind is required",
        )),
    }
}

async fn hold_invoice(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    invoice_id: Uuid,
    reason: String,
) -> Result<AdminRevenueActionResult, Status> {
    let row = sqlx::query_as::<_, (String,)>(
        "UPDATE billing_invoices
         SET internal_hold_at = NOW(), internal_hold_reason = $1,
           internal_hold_by_principal_id = $2, updated_at = NOW()
         WHERE id = $3 AND tenant_id = $4 AND internal_hold_at IS NULL
           AND status::text IN ('draft', 'pro_forma', 'issued')
         RETURNING status::text AS previous_state",
    )
    .bind(&reason)
    .bind(actor_principal_id)
    .bind(invoice_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "invoice_not_holdable: Invoice is missing, belongs to another tenant, already held, or not holdable.",
        )
    })?;

    Ok(result(
        invoice_id,
        "hold_invoice",
        "held",
        "revenue.invoice.held",
        "billing_invoice",
        Some(row.0),
        json!({ "invoice_id": invoice_id }),
    ))
}

async fn release_invoice(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    invoice_id: Uuid,
) -> Result<AdminRevenueActionResult, Status> {
    let row = sqlx::query_as::<_, (String,)>(
        "UPDATE billing_invoices
         SET internal_hold_at = NULL, internal_hold_reason = NULL,
           internal_hold_by_principal_id = NULL, updated_at = NOW()
         WHERE id = $1 AND tenant_id = $2 AND internal_hold_at IS NOT NULL
         RETURNING status::text AS next_state",
    )
    .bind(invoice_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "invoice_not_releasable: Invoice is missing, belongs to another tenant, or is not held.",
        )
    })?;

    Ok(result(
        invoice_id,
        "release_invoice",
        "released",
        "revenue.invoice.released",
        "billing_invoice",
        Some("held".to_string()),
        json!({ "invoice_status": row.0 }),
    ))
}

async fn transition_dunning_case(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    case_id: Uuid,
    next_state: &'static str,
) -> Result<AdminRevenueActionResult, Status> {
    let row = sqlx::query_as::<_, (String, String)>(
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
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "dunning_case_not_transitionable: Dunning case is missing, belongs to another tenant, or already has the requested status.",
        )
    })?;
    let (action_kind, audit_action) = if next_state == "closed" {
        ("close_dunning_case", "revenue.dunning_case.closed")
    } else {
        ("reopen_dunning_case", "revenue.dunning_case.reopened")
    };

    Ok(result(
        case_id,
        action_kind,
        row.1.as_str(),
        audit_action,
        "billing_dunning_case",
        Some(row.0),
        json!({ "dunning_case_id": case_id }),
    ))
}

async fn transition_dispute(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    dispute_id: Uuid,
    next_state: &'static str,
) -> Result<AdminRevenueActionResult, Status> {
    let row = sqlx::query_as::<_, (String, String)>(
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
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "dispute_not_transitionable: Dispute is missing, belongs to another tenant, or already has the requested status.",
        )
    })?;
    let (action_kind, audit_action) = if next_state == "resolved" {
        ("resolve_dispute", "revenue.dispute.resolved")
    } else {
        ("review_dispute", "revenue.dispute.reviewed")
    };

    Ok(result(
        dispute_id,
        action_kind,
        row.1.as_str(),
        audit_action,
        "billing_dispute",
        Some(row.0),
        json!({ "dispute_id": dispute_id }),
    ))
}

fn result(
    object_id: Uuid,
    action_kind: &str,
    status: &str,
    audit_action: &str,
    target_type: &str,
    previous_state: Option<String>,
    metadata: serde_json::Value,
) -> AdminRevenueActionResult {
    AdminRevenueActionResult {
        object_id: object_id.to_string(),
        action_kind: action_kind.to_string(),
        status: status.to_string(),
        audit_action: audit_action.to_string(),
        target_type: target_type.to_string(),
        previous_state: previous_state.unwrap_or_default(),
        metadata_json: metadata.to_string(),
    }
}

fn validate_reason(value: &str) -> Result<(), Status> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(Status::invalid_argument(
        "invalid_revenue_reason: Revenue action reason must contain between 8 and 500 characters.",
    ))
}

#[cfg(test)]
mod tests {
    use super::validate_reason;

    #[test]
    fn revenue_reason_requires_meaningful_text() {
        assert!(validate_reason("ticket REV-123 approved").is_ok());
        assert!(validate_reason("short").is_err());
    }
}
