use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::app::AppState;
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct CommunicationsCenterSnapshot {
    queued_message_count: i64,
    sent_message_count_24h: i64,
    delivered_message_count_24h: i64,
    failed_message_count_24h: i64,
    suppressed_email_count: i64,
    webhook_event_count_24h: i64,
    unprocessed_event_count: i64,
    status_distribution: Vec<EmailStatusDistribution>,
    business_type_distribution: Vec<BusinessTypeDistribution>,
    recent_failures: Vec<RecentEmailFailure>,
    recent_suppressions: Vec<RecentEmailSuppression>,
    recent_unprocessed_events: Vec<RecentEmailEvent>,
}

#[derive(Debug, Serialize)]
struct EmailStatusDistribution {
    status: String,
    message_count: i64,
}

#[derive(Debug, Serialize)]
struct BusinessTypeDistribution {
    business_type: String,
    message_count: i64,
    failure_count: i64,
}

#[derive(Debug, Serialize)]
struct RecentEmailFailure {
    id: uuid::Uuid,
    business_type: String,
    recipient_email: String,
    provider_email_id: Option<String>,
    status: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentEmailSuppression {
    email: String,
    reason: String,
    suppressed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RecentEmailEvent {
    id: uuid::Uuid,
    provider_event_id: String,
    provider_email_id: Option<String>,
    email: String,
    event_type: String,
    occurred_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/communications-center",
        get(communications_center_route),
    )
}

async fn communications_center_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CommunicationsCenterSnapshot>, AppError> {
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(load_communications_center(&state.db).await?))
}

async fn load_communications_center(db: &PgPool) -> Result<CommunicationsCenterSnapshot, AppError> {
    let metrics = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*) FROM email_messages WHERE status::text = 'queued')
            AS queued_message_count,
          (
            SELECT COUNT(*) FROM email_messages
            WHERE status::text IN ('sent', 'delivered')
              AND sent_at >= NOW() - INTERVAL '24 hours'
          ) AS sent_message_count_24h,
          (
            SELECT COUNT(*) FROM email_messages
            WHERE status::text = 'delivered' AND updated_at >= NOW() - INTERVAL '24 hours'
          ) AS delivered_message_count_24h,
          (
            SELECT COUNT(*) FROM email_messages
            WHERE status::text IN ('bounced', 'complained', 'dropped')
              AND updated_at >= NOW() - INTERVAL '24 hours'
          ) AS failed_message_count_24h,
          (SELECT COUNT(*) FROM suppressed_emails) AS suppressed_email_count,
          (
            SELECT COUNT(*) FROM email_events
            WHERE created_at >= NOW() - INTERVAL '24 hours'
          ) AS webhook_event_count_24h,
          (SELECT COUNT(*) FROM email_events WHERE processed_at IS NULL)
            AS unprocessed_event_count
        "#,
    )
    .fetch_one(db)
    .await?;

    Ok(CommunicationsCenterSnapshot {
        queued_message_count: metrics.get("queued_message_count"),
        sent_message_count_24h: metrics.get("sent_message_count_24h"),
        delivered_message_count_24h: metrics.get("delivered_message_count_24h"),
        failed_message_count_24h: metrics.get("failed_message_count_24h"),
        suppressed_email_count: metrics.get("suppressed_email_count"),
        webhook_event_count_24h: metrics.get("webhook_event_count_24h"),
        unprocessed_event_count: metrics.get("unprocessed_event_count"),
        status_distribution: load_status_distribution(db).await?,
        business_type_distribution: load_business_type_distribution(db).await?,
        recent_failures: load_recent_failures(db).await?,
        recent_suppressions: load_recent_suppressions(db).await?,
        recent_unprocessed_events: load_recent_unprocessed_events(db).await?,
    })
}

async fn load_status_distribution(db: &PgPool) -> Result<Vec<EmailStatusDistribution>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT status::text AS status, COUNT(*) AS message_count
        FROM email_messages
        GROUP BY status
        ORDER BY message_count DESC, status ASC
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| EmailStatusDistribution {
            status: row.get("status"),
            message_count: row.get("message_count"),
        })
        .collect())
}

async fn load_business_type_distribution(
    db: &PgPool,
) -> Result<Vec<BusinessTypeDistribution>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT business_type, COUNT(*) AS message_count,
          COUNT(*) FILTER (WHERE status::text IN ('bounced', 'complained', 'dropped'))
            AS failure_count
        FROM email_messages
        GROUP BY business_type
        ORDER BY failure_count DESC, message_count DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| BusinessTypeDistribution {
            business_type: row.get("business_type"),
            message_count: row.get("message_count"),
            failure_count: row.get("failure_count"),
        })
        .collect())
}

async fn load_recent_failures(db: &PgPool) -> Result<Vec<RecentEmailFailure>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, business_type, recipient_email, provider_email_id,
          status::text AS status, updated_at
        FROM email_messages
        WHERE status::text IN ('bounced', 'complained', 'dropped')
        ORDER BY updated_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentEmailFailure {
            id: row.get("id"),
            business_type: row.get("business_type"),
            recipient_email: row.get("recipient_email"),
            provider_email_id: row.get("provider_email_id"),
            status: row.get("status"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

async fn load_recent_suppressions(db: &PgPool) -> Result<Vec<RecentEmailSuppression>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT email, reason, suppressed_at
        FROM suppressed_emails
        ORDER BY suppressed_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentEmailSuppression {
            email: row.get("email"),
            reason: row.get("reason"),
            suppressed_at: row.get("suppressed_at"),
        })
        .collect())
}

async fn load_recent_unprocessed_events(db: &PgPool) -> Result<Vec<RecentEmailEvent>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider_event_id, provider_email_id, email,
          event_type::text AS event_type, occurred_at, created_at
        FROM email_events
        WHERE processed_at IS NULL
        ORDER BY created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RecentEmailEvent {
            id: row.get("id"),
            provider_event_id: row.get("provider_event_id"),
            provider_email_id: row.get("provider_email_id"),
            email: row.get("email"),
            event_type: row.get("event_type"),
            occurred_at: row.get("occurred_at"),
            created_at: row.get("created_at"),
        })
        .collect())
}
