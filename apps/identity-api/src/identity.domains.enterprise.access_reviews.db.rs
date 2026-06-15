use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::types::*;
use crate::http::error::AppError;

#[derive(Debug, FromRow)]
pub struct AccessReviewCampaignRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub starts_at: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub pending_items: i64,
    pub approved_items: i64,
    pub revoked_items: i64,
    pub changed_items: i64,
}

impl AccessReviewCampaignRow {
    pub fn into_view(self) -> AccessReviewCampaignSummary {
        AccessReviewCampaignSummary {
            id: self.id,
            name: self.name,
            description: self.description,
            status: campaign_status_from_db(&self.status),
            starts_at: self.starts_at,
            due_at: self.due_at,
            created_by: self.created_by,
            created_at: self.created_at,
            closed_at: self.closed_at,
            pending_items: self.pending_items,
            approved_items: self.approved_items,
            revoked_items: self.revoked_items,
            changed_items: self.changed_items,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct AccessReviewItemRow {
    pub id: Uuid,
    pub item_type: String,
    pub subject_id: String,
    pub subject_label: String,
    pub workspace_id: Option<Uuid>,
    pub role: Option<String>,
    pub status: String,
    pub evidence: Value,
    pub decision: String,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl AccessReviewItemRow {
    pub fn into_view(self) -> AccessReviewItem {
        AccessReviewItem {
            id: self.id,
            item_type: item_type_from_db(&self.item_type),
            subject_id: self.subject_id,
            subject_label: self.subject_label,
            workspace_id: self.workspace_id,
            role: self.role,
            status: self.status,
            evidence: serde_json::from_value(self.evidence).unwrap_or_default(),
            decision: decision_from_db(&self.decision),
            reviewed_by: self.reviewed_by,
            reviewed_at: self.reviewed_at,
            created_at: self.created_at,
        }
    }
}

pub async fn insert_campaign(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    created_by: Uuid,
    name: &str,
    description: Option<&str>,
    due_at: DateTime<Utc>,
) -> Result<Uuid, AppError> {
    Ok(sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO access_review_campaigns (tenant_id, name, description, due_at, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(name)
    .bind(description)
    .bind(due_at)
    .bind(created_by)
    .fetch_one(&mut **tx)
    .await?)
}

pub async fn insert_snapshot_items(
    tx: &mut Transaction<'_, Postgres>,
    campaign_id: Uuid,
    tenant_id: Uuid,
    scope: &AccessReviewCampaignScopeInput,
) -> Result<u64, AppError> {
    let result = sqlx::query(
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
            COALESCE(u.email, p.display_name) AS subject_label, NULL::uuid AS workspace_id,
            tm.role::text AS role, tm.status::text AS status,
            jsonb_build_object('principal_kind', tm.principal_kind::text, 'created_at', tm.created_at) AS evidence
          FROM tenant_memberships tm
          INNER JOIN principals p ON p.id = tm.principal_id
          LEFT JOIN users u ON u.principal_id = tm.principal_id
          WHERE tm.tenant_id = $2 AND $3

          UNION ALL

          SELECT 'role' AS item_type, concat(wm.principal_id::text, ':', wm.workspace_id::text) AS subject_id,
            concat(COALESCE(u.email, p.display_name), ' in ', w.name) AS subject_label,
            wm.workspace_id, wm.role::text AS role, wm.status::text AS status,
            jsonb_build_object('workspace_name', w.name, 'source', wm.source::text, 'created_at', wm.created_at) AS evidence
          FROM workspace_memberships wm
          INNER JOIN workspaces w ON w.id = wm.workspace_id
          INNER JOIN principals p ON p.id = wm.principal_id
          LEFT JOIN users u ON u.principal_id = wm.principal_id
          WHERE w.tenant_id = $2 AND wm.status IN ('active', 'suspended') AND $4

          UNION ALL

          SELECT 'service_account' AS item_type, sa.principal_id::text AS subject_id,
            sa.name AS subject_label, sa.workspace_id, wm.role::text AS role,
            p.status::text AS status,
            jsonb_build_object('auth_method', sa.auth_method, 'client_id', sa.client_id, 'created_at', sa.created_at) AS evidence
          FROM service_accounts sa
          INNER JOIN principals p ON p.id = sa.principal_id
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
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_campaigns(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<AccessReviewCampaignRow>, AppError> {
    Ok(sqlx::query_as::<_, AccessReviewCampaignRow>(&format!(
        "{}{}",
        campaign_summary_sql(""),
        " ORDER BY arc.created_at DESC"
    ))
    .bind(tenant_id)
    .fetch_all(db)
    .await?)
}

pub async fn get_campaign(
    db: &PgPool,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<Option<AccessReviewCampaignRow>, AppError> {
    Ok(sqlx::query_as::<_, AccessReviewCampaignRow>(&format!(
        "{}{}",
        campaign_summary_sql("AND arc.id = $2"),
        " ORDER BY arc.created_at DESC"
    ))
    .bind(tenant_id)
    .bind(campaign_id)
    .fetch_optional(db)
    .await?)
}

pub async fn list_items(
    db: &PgPool,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<Vec<AccessReviewItemRow>, AppError> {
    Ok(sqlx::query_as::<_, AccessReviewItemRow>(
        r#"
        SELECT id, item_type::text AS item_type, subject_id, subject_label, workspace_id, role,
          status, evidence, decision::text AS decision, reviewed_by, reviewed_at, created_at
        FROM access_review_items
        WHERE tenant_id = $1 AND campaign_id = $2
        ORDER BY item_type::text ASC, subject_label ASC, created_at ASC
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .fetch_all(db)
    .await?)
}

fn campaign_summary_sql(extra_filter: &str) -> String {
    format!(
        r#"
    SELECT arc.id, arc.name, arc.description, arc.status::text AS status, arc.starts_at,
      arc.due_at, arc.created_by, arc.created_at, arc.closed_at,
      COUNT(ari.id) FILTER (WHERE ari.decision = 'pending')::bigint AS pending_items,
      COUNT(ari.id) FILTER (WHERE ari.decision = 'approved')::bigint AS approved_items,
      COUNT(ari.id) FILTER (WHERE ari.decision = 'revoked')::bigint AS revoked_items,
      COUNT(ari.id) FILTER (WHERE ari.decision = 'changed')::bigint AS changed_items
    FROM access_review_campaigns arc
    LEFT JOIN access_review_items ari ON ari.campaign_id = arc.id
    WHERE arc.tenant_id = $1
      {extra_filter}
    GROUP BY arc.id
    "#
    )
}

fn campaign_status_from_db(value: &str) -> AccessReviewCampaignStatus {
    match value {
        "draft" => AccessReviewCampaignStatus::Draft,
        "closed" => AccessReviewCampaignStatus::Closed,
        _ => AccessReviewCampaignStatus::Active,
    }
}

fn item_type_from_db(value: &str) -> AccessReviewItemType {
    match value {
        "role" => AccessReviewItemType::Role,
        "service_account" => AccessReviewItemType::ServiceAccount,
        "oauth_client" => AccessReviewItemType::OAuthClient,
        _ => AccessReviewItemType::Member,
    }
}

fn decision_from_db(value: &str) -> AccessReviewItemDecision {
    match value {
        "approved" => AccessReviewItemDecision::Approved,
        "revoked" => AccessReviewItemDecision::Revoked,
        "changed" => AccessReviewItemDecision::Changed,
        _ => AccessReviewItemDecision::Pending,
    }
}
