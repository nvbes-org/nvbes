use chrono::{DateTime, Duration, Utc};
use nvbes_core::authz::{action_requires_independent_approval, parse_action};
use sqlx::{Postgres, Row, Transaction};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{non_empty, parse_uuid, sql_status},
};

const DEFAULT_APPROVAL_MINUTES: i32 = 5;
const MAX_APPROVAL_MINUTES: i32 = 15;

pub async fn approve(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    approver_id: Uuid,
    request: enterprise::ApprovePrivilegedActionRequest,
) -> Result<enterprise::PrivilegedActionApproval, Status> {
    let requested_by = parse_uuid(&request.requested_by, "requested_by")?;
    if requested_by == approver_id {
        return Err(Status::permission_denied(
            "privileged actions require an independent approver",
        ));
    }
    let action = parse_action(request.action.trim())
        .filter(|action| action_requires_independent_approval(*action))
        .ok_or_else(|| Status::invalid_argument("action does not support dual approval"))?;
    let resource = non_empty(request.resource, "resource")?;
    let reason = non_empty(request.reason, "reason")?;
    let duration = match request.duration_minutes {
        value if value <= 0 => DEFAULT_APPROVAL_MINUTES,
        value => value.min(MAX_APPROVAL_MINUTES),
    };

    let mut tx = db.begin().await.map_err(sql_status)?;
    ensure_approver(&mut tx, tenant_id, approver_id).await?;
    ensure_active_member(&mut tx, tenant_id, requested_by).await?;
    let expires_at = Utc::now() + Duration::minutes(i64::from(duration));
    let row = sqlx::query(
        r#"
        INSERT INTO privileged_action_approvals (
          tenant_id, action, resource, requested_by, approved_by, reason, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (tenant_id, action, resource, requested_by)
          WHERE consumed_at IS NULL
        DO UPDATE SET
          approved_by = EXCLUDED.approved_by,
          reason = EXCLUDED.reason,
          expires_at = EXCLUDED.expires_at
        RETURNING id, tenant_id, requested_by, approved_by, action, resource,
          expires_at, consumed_at
        "#,
    )
    .bind(tenant_id)
    .bind(action.as_str())
    .bind(resource)
    .bind(requested_by)
    .bind(approver_id)
    .bind(reason)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(sql_status)?;
    insert_audit(
        &mut tx,
        tenant_id,
        approver_id,
        requested_by,
        "enterprise.privileged_action.approved",
        serde_json::json!({"action": action.as_str(), "expires_at": expires_at}),
    )
    .await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(from_row(row))
}

pub async fn consume(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    approval_id: Uuid,
) -> Result<enterprise::PrivilegedActionApproval, Status> {
    let mut tx = db.begin().await.map_err(sql_status)?;
    let row = sqlx::query(
        r#"
        UPDATE privileged_action_approvals
        SET consumed_at = NOW()
        WHERE id = $1
          AND tenant_id = $2
          AND requested_by = $3
          AND consumed_at IS NULL
          AND expires_at > NOW()
        RETURNING id, tenant_id, requested_by, approved_by, action, resource,
          expires_at, consumed_at
        "#,
    )
    .bind(approval_id)
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::failed_precondition("approval is missing, expired, or consumed"))?;
    insert_audit(
        &mut tx,
        tenant_id,
        actor_id,
        actor_id,
        "enterprise.privileged_action.consumed",
        serde_json::json!({"approval_id": approval_id}),
    )
    .await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(from_row(row))
}

async fn ensure_approver(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<(), Status> {
    let role = membership_role(tx, tenant_id, principal_id).await?;
    if matches!(role.as_str(), "owner" | "security_admin") {
        Ok(())
    } else {
        Err(Status::permission_denied(
            "owner or security_admin role is required to approve privileged actions",
        ))
    }
}

async fn ensure_active_member(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<(), Status> {
    membership_role(tx, tenant_id, principal_id)
        .await
        .map(|_| ())
}

async fn membership_role(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<String, Status> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM tenant_memberships
        WHERE tenant_id = $1 AND principal_id = $2 AND status = 'active'
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::permission_denied("active tenant membership is required"))
}

async fn insert_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    actor_id: Uuid,
    target_id: Uuid,
    action: &'static str,
    metadata: serde_json::Value,
) -> Result<(), Status> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id, actor_principal_id, action, target_type, target_id,
          metadata, event_hash, created_at
        )
        VALUES ($1, $2, $3, 'principal', $4, $5, gen_random_uuid()::text, NOW())
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(action)
    .bind(target_id)
    .bind(metadata)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(())
}

fn from_row(row: sqlx::postgres::PgRow) -> enterprise::PrivilegedActionApproval {
    enterprise::PrivilegedActionApproval {
        approval_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        requested_by: row.get::<Uuid, _>("requested_by").to_string(),
        approved_by: row.get::<Uuid, _>("approved_by").to_string(),
        action: row.get("action"),
        resource: row.get("resource"),
        expires_at: row.get::<DateTime<Utc>, _>("expires_at").to_rfc3339(),
        consumed_at: row
            .get::<Option<DateTime<Utc>>, _>("consumed_at")
            .map(|value| value.to_rfc3339())
            .unwrap_or_default(),
    }
}
