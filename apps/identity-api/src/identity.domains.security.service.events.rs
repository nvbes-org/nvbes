use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::domains::authz::WorkspaceAccess;
use crate::http::error::AppError;
use sqlx::PgPool;

use super::super::super::identity.domains.security.types::*;

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

async fn fetch_risk_events(
    db: &PgPool,
    workspace_id: Uuid,
    before_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<RiskEventView>, AppError> { super::fetch_risk_events(db, workspace_id, before_id, limit).await }

async fn fetch_billing_webhook_events(
    db: &PgPool,
    workspace_id: Uuid,
    limit: i64,
) -> Result<Vec<BillingWebhookEventView>, AppError> { super::fetch_billing_webhook_events(db, workspace_id, limit).await }

fn render_csv(risk_events: &[RiskEventView], billing_webhook_events: &[BillingWebhookEventView]) -> String { super::render_csv(risk_events, billing_webhook_events) }

fn normalize_limit(limit: Option<i64>) -> i64 { super::normalize_limit(limit) }
