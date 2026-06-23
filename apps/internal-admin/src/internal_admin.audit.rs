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
        list_audit_events(&state.db, access.tenant_id, limit).await?,
    ))
}

async fn list_audit_events(
    db: &PgPool,
    tenant_id: Uuid,
    limit: i64,
) -> Result<Vec<AuditEvent>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT ae.id, ae.action, ae.actor_principal_id, u.email AS actor_email,
          ae.target_type, ae.target_id, ae.metadata, ae.created_at
        FROM audit_events ae
        LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
        WHERE ae.tenant_id = $1
        ORDER BY ae.created_at DESC, ae.id DESC
        LIMIT $2
        "#,
    )
    .bind(tenant_id)
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
            created_at: row.get("created_at"),
        })
        .collect())
}

fn default_limit() -> i64 {
    25
}
