use crate::domains::authz::WorkspaceAccess;
use crate::http::error::AppError;
use nvbes_core::pagination::{KeysetCursor, page_from_rows};
use sqlx::PgPool;

use super::types::*;

#[path = "identity.domains.security.service.risk_events.rs"]
mod risk_events;
#[path = "identity.domains.security.service.summary.rs"]
mod summary;

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
    let cursor = input
        .cursor
        .as_deref()
        .map(KeysetCursor::decode)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let risk_events = fetch_risk_events(db, access.tenant_id, cursor, limit + 1, &filters).await?;
    let page = page_from_rows(risk_events, limit as usize, |event| KeysetCursor {
        created_at: event.created_at,
        id: event.id,
    });

    let summary = security_events_summary(&page.items);
    let events = page
        .items
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
        next_cursor: page
            .next_cursor
            .map(KeysetCursor::encode)
            .transpose()
            .map_err(|_| {
                AppError::internal(
                    "cursor_encoding_failed",
                    "Pagination cursor could not be encoded.",
                )
            })?,
        has_more: page.has_more,
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
        access.tenant_id,
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
