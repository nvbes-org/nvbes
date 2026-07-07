use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{pb::nvbes::enterprise::v1 as enterprise, service_status::sql_status};

const DEFAULT_REMINDER_LIMIT: i64 = 100;
const MAX_REMINDER_LIMIT: i64 = 500;
const REMINDER_WINDOW_DAYS: i32 = 2;

pub async fn claim_reminder_candidates(
    db: &sqlx::PgPool,
    request: enterprise::ClaimAccessReviewReminderCandidatesRequest,
) -> Result<enterprise::AccessReviewReminderClaim, Status> {
    let mut tx = db.begin().await.map_err(sql_status)?;
    let candidates = reminder_candidates(&mut tx, requested_limit(request.limit)).await?;
    let mut claimed = Vec::new();
    for candidate in candidates {
        if record_reminder_claim(&mut tx, &candidate).await? {
            claimed.push(candidate.into_contract());
        }
    }
    tx.commit().await.map_err(sql_status)?;
    Ok(enterprise::AccessReviewReminderClaim {
        candidates: claimed,
    })
}

async fn reminder_candidates(
    tx: &mut Transaction<'_, Postgres>,
    limit: i64,
) -> Result<Vec<ReminderCandidateRow>, Status> {
    sqlx::query_as::<_, ReminderCandidateRow>(
        r#"
        WITH active_campaigns AS (
          SELECT arc.id, arc.tenant_id, arc.name, arc.due_at, t.name AS tenant_name,
            COUNT(ari.id) FILTER (WHERE ari.decision = 'pending')::bigint AS pending_items
          FROM access_review_campaigns arc
          INNER JOIN tenants t ON t.id = arc.tenant_id
          LEFT JOIN access_review_items ari ON ari.campaign_id = arc.id
          WHERE arc.status = 'active'
            AND arc.due_at <= NOW() + ($1::int * INTERVAL '1 day')
          GROUP BY arc.id, t.name
        )
        SELECT ac.tenant_id, ac.id AS campaign_id, ac.name AS campaign_name,
          ac.tenant_name, ac.due_at, ac.pending_items,
          tm.principal_id AS recipient_principal_id,
          u.email AS recipient_email,
          COALESCE(NULLIF(concat_ws(' ', u.firstname, u.lastname), ''), u.username, u.email) AS recipient_name,
          CASE WHEN ac.due_at < NOW() THEN 'overdue' ELSE 'due_soon' END AS reminder_kind
        FROM active_campaigns ac
        INNER JOIN tenant_memberships tm ON tm.tenant_id = ac.tenant_id
        INNER JOIN users u ON u.principal_id = tm.principal_id
        WHERE ac.pending_items > 0
          AND tm.status = 'active'
          AND tm.principal_kind = 'human'
          AND tm.role IN ('owner', 'admin', 'security_admin')
          AND u.status = 'active'
          AND NOT EXISTS (
            SELECT 1
            FROM access_review_reminders arr
            WHERE arr.campaign_id = ac.id
              AND arr.recipient_principal_id = tm.principal_id
              AND arr.reminder_kind = CASE
                WHEN ac.due_at < NOW() THEN 'overdue'
                ELSE 'due_soon'
              END
          )
        ORDER BY ac.due_at ASC, ac.id ASC, tm.role ASC, u.email ASC
        LIMIT $2
        "#,
    )
    .bind(REMINDER_WINDOW_DAYS)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await
    .map_err(sql_status)
}

async fn record_reminder_claim(
    tx: &mut Transaction<'_, Postgres>,
    candidate: &ReminderCandidateRow,
) -> Result<bool, Status> {
    let rows = sqlx::query(
        r#"
        INSERT INTO access_review_reminders (
          tenant_id, campaign_id, recipient_principal_id, reminder_kind
        )
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (campaign_id, recipient_principal_id, reminder_kind) DO NOTHING
        "#,
    )
    .bind(candidate.tenant_id)
    .bind(candidate.campaign_id)
    .bind(candidate.recipient_principal_id)
    .bind(&candidate.reminder_kind)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?
    .rows_affected();
    Ok(rows > 0)
}

fn requested_limit(limit: i32) -> i64 {
    if limit <= 0 {
        DEFAULT_REMINDER_LIMIT
    } else {
        i64::from(limit).min(MAX_REMINDER_LIMIT)
    }
}

#[derive(Debug, FromRow)]
struct ReminderCandidateRow {
    tenant_id: Uuid,
    campaign_id: Uuid,
    campaign_name: String,
    tenant_name: String,
    due_at: DateTime<Utc>,
    pending_items: i64,
    recipient_principal_id: Uuid,
    recipient_email: String,
    recipient_name: String,
    reminder_kind: String,
}

impl ReminderCandidateRow {
    fn into_contract(self) -> enterprise::AccessReviewReminderCandidate {
        enterprise::AccessReviewReminderCandidate {
            tenant_id: self.tenant_id.to_string(),
            campaign_id: self.campaign_id.to_string(),
            campaign_name: self.campaign_name,
            tenant_name: self.tenant_name,
            due_at: self.due_at.to_rfc3339(),
            pending_items: self.pending_items,
            recipient_principal_id: self.recipient_principal_id.to_string(),
            recipient_email: self.recipient_email,
            recipient_name: self.recipient_name,
            reminder_kind: self.reminder_kind,
        }
    }
}

#[cfg(test)]
#[path = "enterprise.grpc.access_reviews.reminders.contract_tests.rs"]
mod contract_tests;
