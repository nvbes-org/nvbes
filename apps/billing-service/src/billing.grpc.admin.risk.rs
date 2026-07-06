use sqlx::Row;
use tonic::Status;

use crate::grpc::pb::nvbes::billing::v1::{
    AdminAccessPolicySnapshot, AdminBillingRiskScore, AdminBillingRiskSignal, AdminRiskActionKind,
    AdminRiskActionResult, AdminRiskDecisionCenterSnapshot,
};

pub async fn risk_decision_center(
    db: &sqlx::PgPool,
) -> Result<AdminRiskDecisionCenterSnapshot, Status> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (
            SELECT COUNT(*) FROM billing_risk_signals
            WHERE occurred_at >= NOW() - INTERVAL '24 hours'
          ) AS billing_risk_signal_count_24h,
          (SELECT COUNT(*) FROM billing_risk_scores WHERE score >= 70)
            AS high_billing_risk_score_count,
          (
            SELECT COUNT(*) FROM billing_access_policy_snapshots
            WHERE effective_from <= NOW() AND (effective_to IS NULL OR effective_to > NOW())
          ) AS active_access_policy_count,
          (
            SELECT COUNT(*) FROM billing_access_policy_snapshots
            WHERE created_at >= NOW() - INTERVAL '24 hours'
          ) AS access_policy_count_24h
        "#,
    )
    .fetch_one(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(AdminRiskDecisionCenterSnapshot {
        billing_risk_signal_count_24h: metrics.get("billing_risk_signal_count_24h"),
        high_billing_risk_score_count: metrics.get("high_billing_risk_score_count"),
        active_access_policy_count: metrics.get("active_access_policy_count"),
        access_policy_count_24h: metrics.get("access_policy_count_24h"),
        billing_risk_scores: load_billing_risk_scores(db).await?,
        billing_risk_signals: load_billing_risk_signals(db).await?,
        active_access_policies: load_access_policies(db).await?,
    })
}

async fn load_billing_risk_scores(db: &sqlx::PgPool) -> Result<Vec<AdminBillingRiskScore>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT brs.id, brs.tenant_id, t.name AS tenant_name, brs.score::float8 AS score,
          brs.decision, brs.created_at
        FROM billing_risk_scores brs
        JOIN tenants t ON t.id = brs.tenant_id
        ORDER BY brs.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminBillingRiskScore {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            score: row.get("score"),
            decision: row.get("decision"),
            created_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect())
}

pub async fn run_risk_action(
    db: &sqlx::PgPool,
    kind: AdminRiskActionKind,
    tenant_id: uuid::Uuid,
    actor_principal_id: uuid::Uuid,
    workspace_id: uuid::Uuid,
    target_id: uuid::Uuid,
    reason: String,
) -> Result<AdminRiskActionResult, Status> {
    validate_reason(&reason)?;
    match kind {
        AdminRiskActionKind::ApprovePolicy => {
            decide_policy(db, tenant_id, workspace_id, target_id, "approved", reason).await
        }
        AdminRiskActionKind::BlockPolicy => {
            decide_policy(db, tenant_id, workspace_id, target_id, "blocked", reason).await
        }
        AdminRiskActionKind::ResolveRiskSignal => {
            resolve_risk_signal(db, tenant_id, actor_principal_id, target_id, reason).await
        }
        AdminRiskActionKind::Unspecified => Err(Status::invalid_argument(
            "admin risk action kind is required",
        )),
    }
}

async fn resolve_risk_signal(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    actor_principal_id: uuid::Uuid,
    signal_id: uuid::Uuid,
    reason: String,
) -> Result<AdminRiskActionResult, Status> {
    let row = sqlx::query(
        "UPDATE billing_risk_signals
         SET signal_value = signal_value || jsonb_build_object(
           'resolution_status', 'resolved',
           'resolution_reason', $1,
           'resolved_by', $2::text,
           'resolved_at', NOW()
         )
         WHERE id = $3 AND (tenant_id = $4 OR tenant_id IS NULL)
           AND COALESCE(signal_value->>'resolution_status', '') <> 'resolved'
         RETURNING signal_value",
    )
    .bind(&reason)
    .bind(actor_principal_id)
    .bind(signal_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "risk_signal_not_resolvable: Risk signal is missing, belongs to another tenant, or is already resolved.",
        )
    })?;

    Ok(action_result(
        signal_id,
        "resolve_risk_signal",
        "resolved",
        "risk.signal.resolved",
        "billing_risk_signal",
        None,
        row.get::<serde_json::Value, _>("signal_value"),
    ))
}

async fn decide_policy(
    db: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    workspace_id: uuid::Uuid,
    policy_id: uuid::Uuid,
    next_state: &'static str,
    reason: String,
) -> Result<AdminRiskActionResult, Status> {
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, policy_state AS previous_state
           FROM billing_access_policy_snapshots
           WHERE id = $3 AND tenant_id = $4
             AND (workspace_id = $5 OR workspace_id IS NULL)
             AND policy_state <> $1
         ),
         updated AS (
           UPDATE billing_access_policy_snapshots aps
           SET policy_state = $1, reason = $2
           FROM previous
           WHERE aps.id = previous.id
           RETURNING aps.policy_state AS next_state
         )
         SELECT previous.previous_state, updated.next_state
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(next_state)
    .bind(&reason)
    .bind(policy_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "risk_policy_not_decidable: Risk policy is missing, belongs to another tenant, or is already in the requested state.",
        )
    })?;
    let action_kind = if next_state == "approved" {
        "approve_risk_policy"
    } else {
        "block_risk_policy"
    };
    let audit_action = if next_state == "approved" {
        "risk.policy.approved"
    } else {
        "risk.policy.blocked"
    };

    Ok(action_result(
        policy_id,
        action_kind,
        row.get::<String, _>("next_state"),
        audit_action,
        "billing_access_policy_snapshot",
        Some(row.get("previous_state")),
        serde_json::json!({ "policy_snapshot_id": policy_id }),
    ))
}

fn action_result(
    object_id: uuid::Uuid,
    action_kind: &str,
    status: impl Into<String>,
    audit_action: &str,
    target_type: &str,
    previous_state: Option<String>,
    metadata: serde_json::Value,
) -> AdminRiskActionResult {
    AdminRiskActionResult {
        object_id: object_id.to_string(),
        action_kind: action_kind.to_string(),
        status: status.into(),
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
        "invalid_risk_reason: Risk action reason must contain between 8 and 500 characters.",
    ))
}

async fn load_billing_risk_signals(
    db: &sqlx::PgPool,
) -> Result<Vec<AdminBillingRiskSignal>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT brs.id, brs.tenant_id, t.name AS tenant_name, brs.signal_type, brs.occurred_at
        FROM billing_risk_signals brs
        LEFT JOIN tenants t ON t.id = brs.tenant_id
        ORDER BY brs.occurred_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminBillingRiskSignal {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row
                .get::<Option<uuid::Uuid>, _>("tenant_id")
                .map(|value| value.to_string())
                .unwrap_or_default(),
            tenant_name: row
                .get::<Option<String>, _>("tenant_name")
                .unwrap_or_default(),
            signal_type: row.get("signal_type"),
            occurred_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("occurred_at")
                .to_rfc3339(),
        })
        .collect())
}

async fn load_access_policies(db: &sqlx::PgPool) -> Result<Vec<AdminAccessPolicySnapshot>, Status> {
    let rows = sqlx::query(
        r#"
        SELECT aps.id, aps.tenant_id, t.name AS tenant_name, aps.workspace_id,
          w.name AS workspace_name, aps.policy_state, aps.reason,
          aps.effective_from, aps.effective_to
        FROM billing_access_policy_snapshots aps
        JOIN tenants t ON t.id = aps.tenant_id
        LEFT JOIN workspaces w ON w.id = aps.workspace_id
        WHERE aps.effective_from <= NOW() AND (aps.effective_to IS NULL OR aps.effective_to > NOW())
        ORDER BY aps.effective_from DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    Ok(rows
        .into_iter()
        .map(|row| AdminAccessPolicySnapshot {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            tenant_id: row.get::<uuid::Uuid, _>("tenant_id").to_string(),
            tenant_name: row.get("tenant_name"),
            workspace_id: row
                .get::<Option<uuid::Uuid>, _>("workspace_id")
                .map(|value| value.to_string())
                .unwrap_or_default(),
            workspace_name: row
                .get::<Option<String>, _>("workspace_name")
                .unwrap_or_default(),
            policy_state: row.get("policy_state"),
            reason: row.get("reason"),
            effective_from: row
                .get::<chrono::DateTime<chrono::Utc>, _>("effective_from")
                .to_rfc3339(),
            effective_to: row
                .get::<Option<chrono::DateTime<chrono::Utc>>, _>("effective_to")
                .map(|value| value.to_rfc3339())
                .unwrap_or_default(),
        })
        .collect())
}
