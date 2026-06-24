use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::billing_admin_access::authorize_backoffice;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct AuditQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    action: Option<String>,
    target_type: Option<String>,
    q: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuditEvent {
    id: Uuid,
    action: String,
    actor_principal_id: Option<Uuid>,
    actor_email: Option<String>,
    target_type: String,
    target_id: Option<Uuid>,
    metadata: Value,
    event_hash: String,
    previous_event_hash: Option<String>,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/workspaces/{workspaceId}/admin/audit-events",
        get(list_audit_events_route),
    )
}

async fn list_audit_events_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEvent>>, AppError> {
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    let limit = query.limit.clamp(1, 100);
    Ok(Json(
        list_audit_events(&state.db, access.tenant_id, query, limit).await?,
    ))
}

async fn list_audit_events(
    db: &PgPool,
    tenant_id: Uuid,
    query: AuditQuery,
    limit: i64,
) -> Result<Vec<AuditEvent>, AppError> {
    let search_pattern = query.q.as_ref().map(|value| format!("%{}%", value.trim()));
    let rows = sqlx::query(
        r#"
        SELECT ae.id, ae.action, ae.actor_principal_id, u.email AS actor_email,
          ae.target_type, ae.target_id, ae.metadata, ae.event_hash, ae.previous_event_hash,
          ae.created_at
        FROM audit_events ae
        LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
        WHERE ae.tenant_id = $1
          AND ($2::text IS NULL OR ae.action = $2)
          AND ($3::text IS NULL OR ae.target_type = $3)
          AND (
            $4::text IS NULL
            OR ae.action ILIKE $4
            OR ae.target_type ILIKE $4
            OR ae.target_id::text ILIKE $4
            OR ae.actor_principal_id::text ILIKE $4
            OR u.email ILIKE $4
          )
        ORDER BY ae.created_at DESC, ae.id DESC
        LIMIT $5
        "#,
    )
    .bind(tenant_id)
    .bind(trimmed_filter(query.action))
    .bind(trimmed_filter(query.target_type))
    .bind(search_pattern)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| AuditEvent {
            id: row.get("id"),
            action: row.get("action"),
            actor_principal_id: row.get("actor_principal_id"),
            actor_email: row.get("actor_email"),
            target_type: row.get("target_type"),
            target_id: row.get("target_id"),
            metadata: row.get("metadata"),
            event_hash: row.get("event_hash"),
            previous_event_hash: row.get("previous_event_hash"),
            created_at: row.get("created_at"),
        })
        .collect())
}

fn default_limit() -> i64 {
    25
}

fn trimmed_filter(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimmed_filter_discards_blank_values() {
        assert_eq!(
            trimmed_filter(Some("  action  ".to_string())),
            Some("action".to_string())
        );
        assert_eq!(trimmed_filter(Some("   ".to_string())), None);
        assert_eq!(trimmed_filter(None), None);
    }
}
