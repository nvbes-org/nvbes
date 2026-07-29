#[path = "enterprise.grpc.access_reviews.changes.rs"]
mod changes;
#[path = "enterprise.grpc.access_reviews.reads.rs"]
pub mod reads;
#[path = "enterprise.grpc.access_reviews.reminders.rs"]
pub mod reminders;
#[path = "enterprise.grpc.access_reviews.revocations.rs"]
mod revocations;
#[path = "enterprise.grpc.access_reviews.schedules.rs"]
pub mod schedules;
#[path = "enterprise.grpc.access_reviews.scope.rs"]
mod scope;

use chrono::{DateTime, Utc};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{non_empty, parse_uuid, sql_status},
};
pub(super) use scope::AccessReviewScope;

pub async fn start_access_review(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: enterprise::StartAccessReviewRequest,
) -> Result<enterprise::AccessReview, Status> {
    let scope = AccessReviewScope::from_contract_request(
        &request.scope,
        request.include_members,
        request.include_roles,
        request.include_service_accounts,
        request.include_oauth_clients,
    )?;
    let due_at = parse_due_at(&request.due_at)?;
    let name = campaign_name(&request.name);
    let description = optional_text(&request.description);
    let response_scope = scope.as_contract_scope().to_string();

    let mut tx = db.begin().await.map_err(sql_status)?;
    let campaign_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO access_review_campaigns (tenant_id, name, description, due_at, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(&name)
    .bind(description)
    .bind(due_at)
    .bind(actor_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(sql_status)?;

    let item_count = insert_snapshot_items(&mut tx, campaign_id, tenant_id, &scope).await?;
    if item_count == 0 {
        return Err(Status::failed_precondition(
            "access review scope did not contain reviewable access",
        ));
    }
    tx.commit().await.map_err(sql_status)?;

    Ok(enterprise::AccessReview {
        review_id: campaign_id.to_string(),
        tenant_id: tenant_id.to_string(),
        scope: response_scope,
        status: "active".to_string(),
        due_at: due_at.to_rfc3339(),
    })
}

pub async fn record_access_review_decision(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: enterprise::RecordAccessReviewDecisionRequest,
) -> Result<enterprise::AccessReviewDecision, Status> {
    let campaign_id = parse_uuid(&request.review_id, "review_id")?;
    let decision = validate_decision(&request.decision)?;
    let note = request.reason.trim();
    let note = if note.is_empty() { None } else { Some(note) };

    let mut tx = db.begin().await.map_err(sql_status)?;
    ensure_active_campaign(&mut tx, tenant_id, campaign_id).await?;
    let item = item_for_request(&mut tx, tenant_id, campaign_id, &request).await?;
    let subject_id = item.subject_id.clone();
    if item.decision != "pending" {
        return Err(Status::failed_precondition(
            "access review item already has a decision",
        ));
    }
    let mut target_id = None;
    if decision == "revoked" {
        target_id = revocations::apply_revocation(&mut tx, tenant_id, &item)
            .await?
            .target_id;
    } else if decision == "changed" {
        let target_role = non_empty(request.target_role.clone(), "target_role")?;
        changes::apply_change(&mut tx, tenant_id, &item, &target_role).await?;
    }
    let decided_at = update_decision(
        &mut tx,
        tenant_id,
        campaign_id,
        item.id,
        actor_id,
        decision,
        note,
    )
    .await?;
    if pending_item_count(&mut tx, tenant_id, campaign_id).await? == 0 {
        close_campaign(&mut tx, tenant_id, campaign_id).await?;
    }
    tx.commit().await.map_err(sql_status)?;

    Ok(enterprise::AccessReviewDecision {
        review_id: campaign_id.to_string(),
        subject_principal_id: subject_id,
        decision: decision.to_string(),
        decided_at: decided_at.to_rfc3339(),
        target_id: target_id.map(|id| id.to_string()).unwrap_or_default(),
    })
}

pub async fn close_access_review(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    request: enterprise::CloseAccessReviewRequest,
) -> Result<enterprise::AccessReview, Status> {
    let campaign_id = parse_uuid(&request.review_id, "review_id")?;
    let note = optional_text(&request.reason);

    let mut tx = db.begin().await.map_err(sql_status)?;
    ensure_active_campaign(&mut tx, tenant_id, campaign_id).await?;
    let pending_items = pending_item_count(&mut tx, tenant_id, campaign_id).await?;
    if pending_items > 0 && note.is_none() {
        return Err(Status::invalid_argument(
            "reason is required when closing an access review with pending items",
        ));
    }
    close_campaign(&mut tx, tenant_id, campaign_id).await?;
    let review = access_review(&mut tx, tenant_id, campaign_id).await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(review)
}

pub(super) async fn insert_snapshot_items(
    tx: &mut Transaction<'_, Postgres>,
    campaign_id: Uuid,
    tenant_id: Uuid,
    scope: &AccessReviewScope,
) -> Result<u64, Status> {
    sqlx::query(
        r#"
        INSERT INTO access_review_items (
          campaign_id, tenant_id, item_type, subject_id, subject_label,
          workspace_id, role, status, evidence
        )
        SELECT $1, $2, snapshot.item_type::access_review_item_type, snapshot.subject_id,
          snapshot.subject_label, snapshot.workspace_id, snapshot.role, snapshot.status,
          snapshot.evidence
        FROM (
          SELECT 'member' AS item_type, tm.principal_id::text AS subject_id,
            p.display_name AS subject_label, NULL::uuid AS workspace_id,
            tm.role::text AS role, tm.status::text AS status,
            jsonb_build_object(
              'principal_kind', tm.principal_kind::text,
              'created_at', tm.created_at,
              'last_activity_at', activity.last_activity_at,
              'inactive_days', FLOOR(
                EXTRACT(EPOCH FROM (NOW() - COALESCE(activity.last_activity_at, tm.created_at)))
                / 86400
              )::integer
            ) AS evidence
          FROM tenant_memberships tm
          INNER JOIN principals p ON p.id = tm.principal_id
          LEFT JOIN LATERAL (
            SELECT MAX(us.last_seen_at) AS last_activity_at
            FROM user_sessions us
            WHERE us.principal_id = tm.principal_id
              AND us.tenant_id = tm.tenant_id
          ) activity ON TRUE
          WHERE tm.tenant_id = $2 AND $3

          UNION ALL

          SELECT 'role' AS item_type, concat(wm.principal_id::text, ':', wm.workspace_id::text) AS subject_id,
            concat(p.display_name, ' in ', w.name) AS subject_label,
            wm.workspace_id, wm.role::text AS role, wm.status::text AS status,
            jsonb_build_object('workspace_name', w.name, 'source', wm.source::text, 'created_at', wm.created_at) AS evidence
          FROM workspace_memberships wm
          INNER JOIN workspaces w ON w.id = wm.workspace_id
          INNER JOIN principals p ON p.id = wm.principal_id
          WHERE w.tenant_id = $2 AND wm.status IN ('active', 'suspended') AND $4

          UNION ALL

          SELECT 'service_account' AS item_type, sa.principal_id::text AS subject_id,
            sa.name AS subject_label, sa.workspace_id, wm.role::text AS role,
            p.status::text AS status,
            jsonb_build_object(
              'auth_method', sa.auth_method,
              'client_id', sa.client_id,
              'created_at', sa.created_at,
              'last_activity_at', oc.last_used_at,
              'inactive_days', FLOOR(
                EXTRACT(EPOCH FROM (NOW() - COALESCE(oc.last_used_at, sa.created_at)))
                / 86400
              )::integer,
              'credential_expires_at', sa.credential_expires_at
            ) AS evidence
          FROM service_accounts sa
          INNER JOIN principals p ON p.id = sa.principal_id
          LEFT JOIN oauth_clients oc ON oc.client_id = sa.client_id
          LEFT JOIN workspace_memberships wm ON wm.principal_id = sa.principal_id AND wm.workspace_id = sa.workspace_id
          WHERE sa.tenant_id = $2 AND $5

          UNION ALL

          SELECT 'oauth_client' AS item_type, oc.client_id AS subject_id,
            oc.name AS subject_label, NULL::uuid AS workspace_id, NULL::text AS role,
            CASE WHEN oc.revoked_at IS NULL THEN 'active' ELSE 'revoked' END AS status,
            jsonb_build_object(
              'client_type', oc.client_type::text,
              'owner_scope_type', oc.owner_scope_type::text,
              'owner_scope_id', oc.owner_scope_id,
              'last_used_at', oc.last_used_at,
              'requires_admin_consent', oc.requires_admin_consent
            ) AS evidence
          FROM oauth_clients oc
          WHERE oc.tenant_id = $2 AND $6
        ) snapshot
        "#,
    )
    .bind(campaign_id)
    .bind(tenant_id)
    .bind(scope.include_members)
    .bind(scope.include_roles)
    .bind(scope.include_service_accounts)
    .bind(scope.include_oauth_clients)
    .execute(&mut **tx)
    .await
    .map(|result| result.rows_affected())
    .map_err(sql_status)
}

async fn ensure_active_campaign(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<(), Status> {
    let status = sqlx::query_scalar::<_, String>(
        r#"
        SELECT status::text
        FROM access_review_campaigns
        WHERE tenant_id = $1 AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("enterprise access review was not found"))?;
    if status != "active" {
        return Err(Status::failed_precondition(
            "only active access reviews can record decisions",
        ));
    }
    Ok(())
}

async fn item_for_subject(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
    subject_id: &str,
) -> Result<AccessReviewItemRow, Status> {
    sqlx::query(
        r#"
        SELECT id, item_type::text AS item_type, subject_id, workspace_id, decision::text AS decision
        FROM access_review_items
        WHERE tenant_id = $1 AND campaign_id = $2 AND subject_id = $3
        ORDER BY created_at ASC
        LIMIT 1
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .bind(subject_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(sql_status)?
    .map(AccessReviewItemRow::from_row)
    .transpose()
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("enterprise access review item was not found"))
}

async fn item_for_id(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
    item_id: Uuid,
) -> Result<AccessReviewItemRow, Status> {
    sqlx::query(
        r#"
        SELECT id, item_type::text AS item_type, subject_id, workspace_id, decision::text AS decision
        FROM access_review_items
        WHERE tenant_id = $1 AND campaign_id = $2 AND id = $3
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .bind(item_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(sql_status)?
    .map(AccessReviewItemRow::from_row)
    .transpose()
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("enterprise access review item was not found"))
}

async fn item_for_request(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
    request: &enterprise::RecordAccessReviewDecisionRequest,
) -> Result<AccessReviewItemRow, Status> {
    let item_id = request.item_id.trim();
    if !item_id.is_empty() {
        return item_for_id(tx, tenant_id, campaign_id, parse_uuid(item_id, "item_id")?).await;
    }
    let subject_id = non_empty(request.subject_principal_id.clone(), "subject_principal_id")?;
    item_for_subject(tx, tenant_id, campaign_id, &subject_id).await
}

async fn update_decision(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
    item_id: Uuid,
    reviewer_id: Uuid,
    decision: &str,
    note: Option<&str>,
) -> Result<DateTime<Utc>, Status> {
    sqlx::query_scalar(
        r#"
        UPDATE access_review_items
        SET decision = $5::access_review_item_decision,
          reviewed_by = $4,
          reviewed_at = NOW(),
          evidence = CASE
            WHEN $6::text IS NULL THEN evidence
            ELSE jsonb_set(evidence, '{review_note}', to_jsonb($6::text), true)
          END
        WHERE tenant_id = $1 AND campaign_id = $2 AND id = $3
        RETURNING reviewed_at
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .bind(item_id)
    .bind(reviewer_id)
    .bind(decision)
    .bind(note)
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)
}

async fn pending_item_count(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<i64, Status> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM access_review_items
        WHERE tenant_id = $1 AND campaign_id = $2 AND decision = 'pending'
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)
}

async fn close_campaign(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<(), Status> {
    sqlx::query(
        r#"
        UPDATE access_review_campaigns
        SET status = 'closed', closed_at = NOW()
        WHERE tenant_id = $1 AND id = $2 AND status = 'active'
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(())
}

async fn access_review(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<enterprise::AccessReview, Status> {
    let row = sqlx::query(
        r#"
        SELECT id, tenant_id, status::text AS status, due_at
        FROM access_review_campaigns
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("enterprise access review was not found"))?;
    Ok(enterprise::AccessReview {
        review_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        scope: String::new(),
        status: row.get("status"),
        due_at: row.get::<DateTime<Utc>, _>("due_at").to_rfc3339(),
    })
}

fn parse_due_at(value: &str) -> Result<DateTime<Utc>, Status> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| Status::invalid_argument("due_at must be an RFC3339 timestamp"))
}

fn campaign_name(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        format!("Enterprise access review {}", Utc::now().format("%Y-%m-%d"))
    } else {
        value.to_string()
    }
}

fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn validate_decision(value: &str) -> Result<&'static str, Status> {
    match value.trim() {
        "approved" => Ok("approved"),
        "revoked" => Ok("revoked"),
        "changed" => Ok("changed"),
        _ => Err(Status::invalid_argument(
            "decision must be approved, revoked, or changed",
        )),
    }
}

pub(super) struct AccessReviewItemRow {
    id: Uuid,
    pub(super) item_type: String,
    pub(super) subject_id: String,
    pub(super) workspace_id: Option<Uuid>,
    decision: String,
}

impl AccessReviewItemRow {
    fn from_row(row: PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            item_type: row.try_get("item_type")?,
            subject_id: row.try_get("subject_id")?,
            workspace_id: row.try_get("workspace_id")?,
            decision: row.try_get("decision")?,
        })
    }
}

#[cfg(test)]
#[path = "enterprise.grpc.access_reviews.contract_tests.rs"]
mod contract_tests;
