use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::domains::authz::WorkspaceAccess;
use crate::http::error::AppError;
use sqlx::PgPool;

use super::types::*;

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;

pub async fn list_events(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: ListSecurityEventsInput,
) -> Result<SecurityEventsResponse, AppError> {
    let limit = normalize_limit(input.limit);
    let risk_events = fetch_risk_events(db, access.workspace_id, input.before_id, limit).await?;

    let events = risk_events
        .into_iter()
        .map(|event| SecurityEventView {
            id: event.id,
            event_type: event.event_type,
            created_at: event.created_at,
            ip_address: event.ip_address,
            user_agent: None,
            status: Some(event.decision),
        })
        .collect::<Vec<_>>();

    Ok(SecurityEventsResponse {
        next_cursor: events.last().map(|event| event.id),
        events,
    })
}

pub async fn export_events(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<SecurityExportResponse, AppError> {
    let risk_events = fetch_risk_events(db, access.workspace_id, None, DEFAULT_LIMIT).await?;
    let billing_webhook_events =
        fetch_billing_webhook_events(db, access.workspace_id, DEFAULT_LIMIT).await?;

    Ok(SecurityExportResponse {
        workspace_id: access.workspace_id,
        filename: format!("nvbes-security-{}.csv", access.workspace_id),
        content_type: "text/csv; charset=utf-8",
        body: render_csv(&risk_events, &billing_webhook_events),
    })
}

pub async fn list_recovery_reviews(
    db: &PgPool,
    access: &WorkspaceAccess,
    limit: Option<i64>,
    before_created_at: Option<chrono::DateTime<chrono::Utc>>,
    before_id: Option<Uuid>,
) -> Result<ListRecoveryReviewsResponse, AppError> {
    let limit = normalize_limit(limit);
    let rows = sqlx::query(
        r#"
        SELECT
          req.id,
          req.principal_id,
          req.email,
          req.status,
          req.available_at,
          req.approved_by_principal_id,
          req.approved_at,
          req.review_available_at,
          req.secondary_approved_by_principal_id,
          req.secondary_approved_at,
          req.created_at,
          req.updated_at
        FROM enterprise_password_recovery_requests req
        WHERE req.tenant_id = (
          SELECT tenant_id
          FROM workspaces
          WHERE id = $1
        )
          AND (
            $2::timestamptz IS NULL
            OR req.created_at < $2
            OR (req.created_at = $2 AND ($3::uuid IS NULL OR req.id < $3))
          )
        ORDER BY req.created_at DESC, req.id DESC
        LIMIT $4
        "#,
    )
    .bind(access.workspace_id)
    .bind(before_created_at)
    .bind(before_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let requests = rows
        .into_iter()
        .map(|row| {
            Ok(RecoveryReviewView {
                request_id: row.get("id"),
                principal_id: row.get("principal_id"),
                email: row.get("email"),
                status: row.get("status"),
                available_at: row.get("available_at"),
                approved_by_principal_id: row.get("approved_by_principal_id"),
                approved_at: row.get("approved_at"),
                review_available_at: row.get("review_available_at"),
                secondary_approved_by_principal_id: row.get("secondary_approved_by_principal_id"),
                secondary_approved_at: row.get("secondary_approved_at"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(ListRecoveryReviewsResponse {
        workspace_id: access.workspace_id,
        requests,
    })
}

pub async fn worker_queue_status(
    redis: &nvbes_redis::RedisPool,
    workspace_id: Uuid,
) -> Result<WorkerQueueStatusResponse, AppError> {
    let statuses = nvbes_redis::worker_queue::queue_status(redis, "billing.stripe.webhook.process")
        .await
        .map_err(|err| AppError::internal("redis_worker_queue_status_failed", err.to_string()))?
        .into_iter()
        .map(|entry| WorkerQueueStatusView {
            status: entry.status,
            depth: entry.depth,
            oldest_age_seconds: entry.oldest_age_seconds,
        })
        .collect();

    Ok(WorkerQueueStatusResponse {
        workspace_id,
        queue_name: "billing.stripe.webhook.process".to_string(),
        snapshot_at: chrono::Utc::now(),
        statuses,
    })
}

async fn fetch_risk_events(
    db: &PgPool,
    workspace_id: Uuid,
    before_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<RiskEventView>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT
          id, principal_id, session_id, device_id, event_type, ip_address::text AS ip_address,
          user_agent, risk_score, risk_factors, decision, metadata, created_at
        FROM risk_events
        WHERE principal_id IN (
          SELECT principal_id
          FROM principals
          WHERE tenant_id = (SELECT tenant_id FROM workspaces WHERE id = $1)
        )
          AND ($2::uuid IS NULL OR id < $2)
        ORDER BY created_at DESC, id DESC
        LIMIT $3
        "#,
    )
    .bind(workspace_id)
    .bind(before_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|row| {
            let risk_factors: sqlx::types::Json<Value> = row.get("risk_factors");
            let metadata: sqlx::types::Json<Value> = row.get("metadata");
            Ok(RiskEventView {
                id: row.get("id"),
                principal_id: row.get("principal_id"),
                session_id: row.get("session_id"),
                device_id: row.get("device_id"),
                event_type: row.get("event_type"),
                ip_address: row.get("ip_address"),
                user_agent: row.get("user_agent"),
                risk_score: row.get("risk_score"),
                risk_factors: risk_factors.0,
                decision: row.get("decision"),
                metadata: metadata.0,
                created_at: row.get("created_at"),
            })
        })
        .collect()
}

async fn fetch_billing_webhook_events(
    db: &PgPool,
    workspace_id: Uuid,
    limit: i64,
) -> Result<Vec<BillingWebhookEventView>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT
          bwe.provider_event_id,
          bwe.provider,
          bwe.status,
          bwe.signature_valid,
          bwe.received_at,
          bwe.processed_at
        FROM billing_webhook_events bwe
        WHERE bwe.workspace_id = $1
        ORDER BY bwe.received_at DESC, bwe.provider_event_id DESC
        LIMIT $2
        "#,
    )
    .bind(workspace_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(BillingWebhookEventView {
                provider_event_id: row.get("provider_event_id"),
                provider: row.get("provider"),
                status: row.get("status"),
                signature_valid: row.get("signature_valid"),
                received_at: row.get("received_at"),
                processed_at: row.get("processed_at"),
            })
        })
        .collect()
}

fn render_csv(
    risk_events: &[RiskEventView],
    billing_webhook_events: &[BillingWebhookEventView],
) -> String {
    let mut out = String::from("kind,id,field_a,field_b,field_c,created_at\n");
    for event in risk_events {
        out.push_str(&format!(
            "risk,{},{},{},{},{}\n",
            event.id, event.event_type, event.decision, event.risk_score, event.created_at
        ));
    }
    for event in billing_webhook_events {
        out.push_str(&format!(
            "billing_webhook,{},{},{},{},{}\n",
            event.provider_event_id,
            event.provider,
            event.status,
            event.signature_valid,
            event.received_at
        ));
    }
    out
}

fn normalize_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}
