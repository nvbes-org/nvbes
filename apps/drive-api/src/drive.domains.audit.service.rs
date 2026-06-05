use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{domains::authz::WorkspaceAccess, http::error::AppError};

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;
const EXPORT_LIMIT: i64 = 10_000;

#[derive(Debug, Deserialize)]
pub struct ListAuditEventsInput {
    pub limit: Option<i64>,
    pub before_id: Option<Uuid>,
    pub action: Option<String>,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditEventsResponse {
    pub workspace_id: Uuid,
    pub events: Vec<AuditEventView>,
    pub next_before: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditExportResponse {
    pub workspace_id: Uuid,
    pub filename: String,
    pub content_type: &'static str,
    pub body: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditEventView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Value,
    pub previous_event_hash: Option<String>,
    pub event_hash: String,
    pub created_at: DateTime<Utc>,
}

pub struct AuditRecordInput<'a> {
    pub workspace_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_principal_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: Value,
}

pub async fn list_events(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: ListAuditEventsInput,
) -> Result<AuditEventsResponse, AppError> {
    let limit = normalize_limit(input.limit);
    let rows = fetch_events(db, access.workspace_id, input, limit).await?;
    let next_before = if rows.len() == limit as usize {
        rows.last().map(|event| event.id)
    } else {
        None
    };

    Ok(AuditEventsResponse {
        workspace_id: access.workspace_id,
        events: rows,
        next_before,
    })
}

pub async fn export_events(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<AuditExportResponse, AppError> {
    let rows = fetch_events(
        db,
        access.workspace_id,
        ListAuditEventsInput {
            limit: Some(EXPORT_LIMIT),
            before_id: None,
            action: None,
            actor_user_id: None,
            actor_principal_id: None,
        },
        EXPORT_LIMIT,
    )
    .await?;

    Ok(AuditExportResponse {
        workspace_id: access.workspace_id,
        filename: format!("nvbes-audit-{}.csv", access.workspace_id),
        content_type: "text/csv; charset=utf-8",
        body: events_to_csv(&rows),
    })
}

pub async fn record_event(db: &PgPool, input: AuditRecordInput<'_>) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_events (
          workspace_id,
          actor_user_id,
          actor_principal_id,
          action,
          target_type,
          target_id,
          ip,
          user_agent,
          metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6::inet, $7, $8)
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.actor_user_id)
    .bind(input.actor_principal_id)
    .bind(input.action)
    .bind(input.target_type)
    .bind(input.target_id)
    .bind(input.ip)
    .bind(input.user_agent)
    .bind(sqlx::types::Json(input.metadata))
    .execute(db)
    .await?;

    Ok(())
}

async fn fetch_events(
    db: &PgPool,
    workspace_id: Uuid,
    input: ListAuditEventsInput,
    limit: i64,
) -> Result<Vec<AuditEventView>, AppError> {
    let before_created_at = if let Some(before_id) = input.before_id {
        sqlx::query_scalar::<_, DateTime<Utc>>(
            "SELECT created_at FROM audit_events WHERE id = $1 AND workspace_id = $2",
        )
        .bind(before_id)
        .bind(workspace_id)
        .fetch_optional(db)
        .await?
    } else {
        None
    };

    let rows = sqlx::query(
        r#"
        SELECT
          ae.id,
          ae.workspace_id,
          ae.actor_user_id,
          ae.actor_principal_id,
          u.email AS actor_email,
          ae.action,
          ae.target_type,
          ae.target_id,
          ae.ip::text AS ip,
          ae.user_agent,
          ae.metadata,
          ae.previous_event_hash,
          ae.event_hash,
          ae.created_at
        FROM audit_events ae
        LEFT JOIN users u ON u.id = ae.actor_user_id
        WHERE ae.workspace_id = $1
          AND ($2::timestamptz IS NULL OR ae.created_at < $2)
          AND ($3::text IS NULL OR ae.action = $3)
          AND ($4::uuid IS NULL OR ae.actor_user_id = $4)
          AND ($5::uuid IS NULL OR ae.actor_principal_id = $5)
        ORDER BY ae.created_at DESC, ae.id DESC
        LIMIT $6
        "#,
    )
    .bind(workspace_id)
    .bind(before_created_at)
    .bind(normalize_optional_text(input.action))
    .bind(input.actor_user_id)
    .bind(input.actor_principal_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|row| {
            let metadata: sqlx::types::Json<Value> = row.get("metadata");
            Ok(AuditEventView {
                id: row.get("id"),
                workspace_id: row.get("workspace_id"),
                actor_user_id: row.get("actor_user_id"),
                actor_principal_id: row.get("actor_principal_id"),
                actor_email: row.get("actor_email"),
                action: row.get("action"),
                target_type: row.get("target_type"),
                target_id: row.get("target_id"),
                ip: row.get("ip"),
                user_agent: row.get("user_agent"),
                metadata: metadata.0,
                previous_event_hash: row.get("previous_event_hash"),
                event_hash: row.get("event_hash"),
                created_at: row.get("created_at"),
            })
        })
        .collect()
}

fn normalize_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn events_to_csv(events: &[AuditEventView]) -> String {
    let mut csv = String::from(
        "id,workspace_id,created_at,actor_user_id,actor_principal_id,actor_email,action,target_type,target_id,ip,user_agent,metadata,previous_event_hash,event_hash\n",
    );

    for event in events {
        let columns = [
            event.id.to_string(),
            event.workspace_id.to_string(),
            event.created_at.to_rfc3339(),
            event
                .actor_user_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            event
                .actor_principal_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            event.actor_email.clone().unwrap_or_default(),
            event.action.clone(),
            event.target_type.clone(),
            event.target_id.map(|id| id.to_string()).unwrap_or_default(),
            event.ip.clone().unwrap_or_default(),
            event.user_agent.clone().unwrap_or_default(),
            event.metadata.to_string(),
            event.previous_event_hash.clone().unwrap_or_default(),
            event.event_hash.clone(),
        ];

        csv.push_str(
            &columns
                .iter()
                .map(|column| csv_escape(column))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }

    csv
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_to_csv_includes_actor_principal_id_column() {
        let csv = events_to_csv(&[AuditEventView {
            id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            actor_user_id: Some(Uuid::new_v4()),
            actor_principal_id: Some(Uuid::new_v4()),
            actor_email: Some("user@example.com".to_string()),
            action: "audit.test".to_string(),
            target_type: "workspace".to_string(),
            target_id: Some(Uuid::new_v4()),
            ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            metadata: serde_json::json!({"ok": true}),
            previous_event_hash: Some("prev".to_string()),
            event_hash: "hash".to_string(),
            created_at: Utc::now(),
        }]);

        let header = csv.lines().next().expect("csv header");
        assert!(header.contains("actor_principal_id"));
        assert!(csv.contains("user@example.com"));
    }
}
