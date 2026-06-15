use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::app::AppConfig;
use crate::email::jobs::{EmailSendPayload, enqueue_email_job_tx};
use crate::email::templates::html_escape;
use crate::http::error::AppError;

const REMINDER_WINDOW_DAYS: i64 = 2;

#[derive(Debug, Default)]
pub struct AccessReviewReminderRun {
    pub reminders_enqueued: u64,
}

pub async fn enqueue_due_campaign_reminders(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    limit: i64,
) -> Result<AccessReviewReminderRun, AppError> {
    let candidates = reminder_candidates(db, limit).await?;
    let mut run = AccessReviewReminderRun::default();

    for candidate in candidates {
        let payload = reminder_email_payload(config, &candidate);
        enqueue_email_job_tx(
            db,
            redis,
            payload,
            &format!(
                "access-review-reminder:{}:{}:{}",
                candidate.campaign_id, candidate.recipient_principal_id, candidate.reminder_kind
            ),
        )
        .await?;
        if record_reminder_sent(db, &candidate).await? {
            run.reminders_enqueued += 1;
        }
    }

    Ok(run)
}

async fn reminder_candidates(
    db: &PgPool,
    limit: i64,
) -> Result<Vec<AccessReviewReminderCandidate>, AppError> {
    Ok(sqlx::query_as::<_, AccessReviewReminderCandidate>(
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
    .bind(REMINDER_WINDOW_DAYS as i32)
    .bind(limit)
    .fetch_all(db)
    .await?)
}

async fn record_reminder_sent(
    db: &PgPool,
    candidate: &AccessReviewReminderCandidate,
) -> Result<bool, AppError> {
    Ok(sqlx::query(
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
    .execute(db)
    .await?
    .rows_affected()
        > 0)
}

fn reminder_email_payload(
    config: &AppConfig,
    candidate: &AccessReviewReminderCandidate,
) -> EmailSendPayload {
    let link = format!(
        "{}/access-reviews",
        config.web_base_url.trim_end_matches('/')
    );
    let subject = if candidate.reminder_kind == "overdue" {
        format!("Access review overdue: {}", candidate.campaign_name)
    } else {
        format!("Access review due soon: {}", candidate.campaign_name)
    };
    let text_body = format!(
        "{}\n\nTenant: {}\nPending items: {}\nDue: {}\n\nOpen access reviews: {}",
        subject, candidate.tenant_name, candidate.pending_items, candidate.due_at, link
    );
    let html_body = format!(
        "<p>{}</p><p><strong>Tenant:</strong> {}<br><strong>Pending items:</strong> {}<br><strong>Due:</strong> {}</p><p><a href=\"{}\">Open access reviews</a></p>",
        html_escape(&subject),
        html_escape(&candidate.tenant_name),
        candidate.pending_items,
        html_escape(&candidate.due_at.to_rfc3339()),
        html_escape(&link)
    );

    EmailSendPayload {
        to_email: candidate.recipient_email.clone(),
        to_name: Some(candidate.recipient_name.clone()),
        subject,
        html_body,
        text_body: Some(text_body),
        business_type: "access_review_reminder".to_string(),
    }
}

#[derive(Debug, FromRow)]
struct AccessReviewReminderCandidate {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminder_email_escapes_campaign_fields() {
        let config = AppConfig {
            web_base_url: "https://identity.example.test".to_string(),
            ..AppConfig::default()
        };
        let candidate = AccessReviewReminderCandidate {
            tenant_id: Uuid::new_v4(),
            campaign_id: Uuid::new_v4(),
            campaign_name: "<Q2>".to_string(),
            tenant_name: "Tenant & Co".to_string(),
            due_at: Utc::now(),
            pending_items: 3,
            recipient_principal_id: Uuid::new_v4(),
            recipient_email: "owner@example.test".to_string(),
            recipient_name: "Owner".to_string(),
            reminder_kind: "due_soon".to_string(),
        };

        let payload = reminder_email_payload(&config, &candidate);

        assert_eq!(payload.business_type, "access_review_reminder");
        assert!(payload.html_body.contains("&lt;Q2&gt;"));
        assert!(payload.html_body.contains("Tenant &amp; Co"));
        assert!(payload.text_body.unwrap().contains("Pending items: 3"));
    }
}
