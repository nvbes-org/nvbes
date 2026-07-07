use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{parse_uuid, sql_status},
};

pub async fn list_access_reviews(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<enterprise::ListAccessReviewsResponse, Status> {
    let campaigns = campaign_rows(db, tenant_id, None)
        .await?
        .into_iter()
        .map(CampaignRow::into_contract)
        .collect();
    Ok(enterprise::ListAccessReviewsResponse { campaigns })
}

pub async fn get_access_review(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    review_id: &str,
) -> Result<enterprise::AccessReviewDetail, Status> {
    let review_id = parse_uuid(review_id, "review_id")?;
    let campaign = campaign_rows(db, tenant_id, Some(review_id))
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| Status::not_found("enterprise access review was not found"))?
        .into_contract();
    let items = item_rows(db, tenant_id, review_id)
        .await?
        .into_iter()
        .map(ItemRow::into_contract)
        .collect();
    Ok(enterprise::AccessReviewDetail {
        campaign: Some(campaign),
        items,
    })
}

async fn campaign_rows(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    review_id: Option<Uuid>,
) -> Result<Vec<CampaignRow>, Status> {
    let sql = format!(
        "{}{}",
        campaign_summary_sql(review_id.is_some()),
        " ORDER BY arc.created_at DESC"
    );
    let query = sqlx::query_as::<_, CampaignRow>(&sql).bind(tenant_id);
    let query = if let Some(review_id) = review_id {
        query.bind(review_id)
    } else {
        query
    };
    query.fetch_all(db).await.map_err(sql_status)
}

async fn item_rows(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    review_id: Uuid,
) -> Result<Vec<ItemRow>, Status> {
    sqlx::query_as::<_, ItemRow>(
        r#"
        SELECT id, item_type::text AS item_type, subject_id, subject_label, workspace_id, role,
          status, evidence, decision::text AS decision, reviewed_by, reviewed_at, created_at
        FROM access_review_items
        WHERE tenant_id = $1 AND campaign_id = $2
        ORDER BY item_type::text ASC, subject_label ASC, created_at ASC
        "#,
    )
    .bind(tenant_id)
    .bind(review_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)
}

fn campaign_summary_sql(filter_by_review: bool) -> String {
    let extra_filter = if filter_by_review {
        "AND arc.id = $2"
    } else {
        ""
    };
    format!(
        r#"
        SELECT arc.id, arc.tenant_id, arc.name, arc.description, arc.status::text AS status,
          arc.starts_at, arc.due_at, arc.created_by, arc.created_at, arc.closed_at,
          COUNT(ari.id) FILTER (WHERE ari.decision = 'pending')::bigint AS pending_items,
          COUNT(ari.id) FILTER (WHERE ari.decision = 'approved')::bigint AS approved_items,
          COUNT(ari.id) FILTER (WHERE ari.decision = 'revoked')::bigint AS revoked_items,
          COUNT(ari.id) FILTER (WHERE ari.decision = 'changed')::bigint AS changed_items,
          COALESCE(reminders.due_soon_reminders_sent, 0)::bigint AS due_soon_reminders_sent,
          COALESCE(reminders.overdue_reminders_sent, 0)::bigint AS overdue_reminders_sent,
          reminders.last_reminder_at
        FROM access_review_campaigns arc
        LEFT JOIN access_review_items ari ON ari.campaign_id = arc.id
        LEFT JOIN LATERAL (
          SELECT
            COUNT(*) FILTER (WHERE arr.reminder_kind = 'due_soon')::bigint AS due_soon_reminders_sent,
            COUNT(*) FILTER (WHERE arr.reminder_kind = 'overdue')::bigint AS overdue_reminders_sent,
            MAX(arr.created_at) AS last_reminder_at
          FROM access_review_reminders arr
          WHERE arr.campaign_id = arc.id
        ) reminders ON true
        WHERE arc.tenant_id = $1
          {extra_filter}
        GROUP BY arc.id, reminders.due_soon_reminders_sent, reminders.overdue_reminders_sent,
          reminders.last_reminder_at
        "#
    )
}

#[derive(Debug, FromRow)]
struct CampaignRow {
    id: Uuid,
    tenant_id: Uuid,
    name: String,
    description: Option<String>,
    status: String,
    starts_at: DateTime<Utc>,
    due_at: DateTime<Utc>,
    created_by: Uuid,
    created_at: DateTime<Utc>,
    closed_at: Option<DateTime<Utc>>,
    pending_items: i64,
    approved_items: i64,
    revoked_items: i64,
    changed_items: i64,
    due_soon_reminders_sent: i64,
    overdue_reminders_sent: i64,
    last_reminder_at: Option<DateTime<Utc>>,
}

impl CampaignRow {
    fn into_contract(self) -> enterprise::AccessReviewSummary {
        enterprise::AccessReviewSummary {
            review_id: self.id.to_string(),
            tenant_id: self.tenant_id.to_string(),
            name: self.name,
            description: self.description.unwrap_or_default(),
            status: self.status,
            starts_at: self.starts_at.to_rfc3339(),
            due_at: self.due_at.to_rfc3339(),
            created_by: self.created_by.to_string(),
            created_at: self.created_at.to_rfc3339(),
            closed_at: self.closed_at.map(time_string).unwrap_or_default(),
            pending_items: self.pending_items,
            approved_items: self.approved_items,
            revoked_items: self.revoked_items,
            changed_items: self.changed_items,
            due_soon_reminders_sent: self.due_soon_reminders_sent,
            overdue_reminders_sent: self.overdue_reminders_sent,
            last_reminder_at: self.last_reminder_at.map(time_string).unwrap_or_default(),
        }
    }
}

#[derive(Debug, FromRow)]
struct ItemRow {
    id: Uuid,
    item_type: String,
    subject_id: String,
    subject_label: String,
    workspace_id: Option<Uuid>,
    role: Option<String>,
    status: String,
    evidence: Value,
    decision: String,
    reviewed_by: Option<Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl ItemRow {
    fn into_contract(self) -> enterprise::AccessReviewItem {
        enterprise::AccessReviewItem {
            item_id: self.id.to_string(),
            item_type: self.item_type,
            subject_id: self.subject_id,
            subject_label: self.subject_label,
            workspace_id: self
                .workspace_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            role: self.role.unwrap_or_default(),
            status: self.status,
            evidence_json: self.evidence.to_string(),
            decision: self.decision,
            reviewed_by: self
                .reviewed_by
                .map(|id| id.to_string())
                .unwrap_or_default(),
            reviewed_at: self.reviewed_at.map(time_string).unwrap_or_default(),
            created_at: self.created_at.to_rfc3339(),
        }
    }
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
