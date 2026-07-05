use sqlx::Row;
use uuid::Uuid;

use crate::domains::authz::WorkspaceAccess;
use crate::http::error::AppError;
use sqlx::PgPool;

use super::types::*;

#[path = "identity.domains.security.service.risk_events.rs"]
mod risk_events;
#[path = "identity.domains.security.service.summary.rs"]
mod summary;

use crate::email::jobs::JOB_EMAIL_SEND;
use risk_events::{SecurityEventFilters, fetch_risk_events};
use summary::security_events_summary;

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;

pub async fn list_events(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: ListSecurityEventsInput,
) -> Result<SecurityEventsResponse, AppError> {
    let limit = normalize_limit(input.limit);
    let filters = SecurityEventFilters::from_input(&input);
    let risk_events =
        fetch_risk_events(db, access.workspace_id, input.before_id, limit, &filters).await?;

    let summary = security_events_summary(&risk_events);
    let events = risk_events
        .iter()
        .map(|event| SecurityEventView {
            id: event.id,
            event_type: event.event_type.clone(),
            created_at: event.created_at,
            ip_address: event.ip_address.clone(),
            user_agent: None,
            status: Some(event.decision.clone()),
            risk_score: Some(event.risk_score),
            geo_country_code: event.geo_country_code.clone(),
            geo_source: event.geo_source.clone(),
            geo_confidence: event.geo_confidence.clone(),
            geo_network_kind: event.geo_network_kind.clone(),
            geo_risk_score: event.geo_risk_score,
            geo_risk_labels: event.geo_risk_labels.clone(),
        })
        .collect::<Vec<_>>();

    Ok(SecurityEventsResponse {
        next_cursor: events.last().map(|event| event.id),
        events,
        summary,
    })
}

pub async fn export_events(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<SecurityExportResponse, AppError> {
    let risk_events = fetch_risk_events(
        db,
        access.workspace_id,
        None,
        DEFAULT_LIMIT,
        &SecurityEventFilters::default(),
    )
    .await?;

    Ok(SecurityExportResponse {
        workspace_id: access.workspace_id,
        filename: format!("nvbes-security-{}.csv", access.workspace_id),
        content_type: "text/csv; charset=utf-8",
        body: render_csv(&risk_events),
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
    let statuses = nvbes_redis::worker_queue::queue_status(redis, JOB_EMAIL_SEND)
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
        queue_name: JOB_EMAIL_SEND.to_string(),
        snapshot_at: chrono::Utc::now(),
        statuses,
    })
}

fn render_csv(risk_events: &[RiskEventView]) -> String {
    let mut out = String::from(
        "kind,id,event_type,status,risk_score,geo_network_kind,geo_risk_score,geo_risk_labels,created_at\n",
    );
    for event in risk_events {
        out.push_str(&format!(
            "risk,{},{},{},{},{},{},{},{}\n",
            event.id,
            csv_cell(&event.event_type),
            csv_cell(&event.decision),
            event.risk_score,
            csv_cell(event.geo_network_kind.as_deref().unwrap_or("")),
            event
                .geo_risk_score
                .map(|score| score.to_string())
                .unwrap_or_default(),
            csv_cell(&event.geo_risk_labels.join("|")),
            event.created_at
        ));
    }
    out
}

fn csv_cell(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn normalize_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}
