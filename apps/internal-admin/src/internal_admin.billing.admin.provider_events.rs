use axum::{Json, Router, extract::Path, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::authorize_backoffice;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct ProviderEventFailure {
    id: Uuid,
    provider: String,
    provider_event_id: String,
    event_type: String,
    status: String,
    signature_valid: bool,
    payload_summary: Value,
    received_at: DateTime<Utc>,
    processed_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/workspaces/{workspaceId}/billing/admin/provider-events/failures",
        get(list_provider_event_failures_route),
    )
}

async fn list_provider_event_failures_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<ProviderEventFailure>>, AppError> {
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    Ok(Json(
        list_provider_event_failures(&state.db, access.tenant_id).await?,
    ))
}

async fn list_provider_event_failures(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<ProviderEventFailure>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider::text, provider_event_id, event_type, status::text,
          signature_valid, payload_summary, received_at, processed_at
        FROM billing_provider_events
        WHERE tenant_id = $1 AND status::text IN ('failed', 'rejected')
        ORDER BY received_at DESC, id DESC
        LIMIT 50
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ProviderEventFailure {
            id: row.get(0),
            provider: row.get(1),
            provider_event_id: row.get(2),
            event_type: row.get(3),
            status: row.get(4),
            signature_valid: row.get(5),
            payload_summary: row.get(6),
            received_at: row.get(7),
            processed_at: row.get(8),
        })
        .collect())
}
