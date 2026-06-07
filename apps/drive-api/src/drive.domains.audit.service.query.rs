use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domains::audit::service::events_to_csv, domains::authz::WorkspaceAccess, http::error::AppError,
};

use super::{
    AuditEventView, AuditEventsResponse, AuditExportResponse, AuditRecordInput, EXPORT_LIMIT,
    ListAuditEventsInput, normalize_limit, normalize_optional_text,
};

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
            let metadata: sqlx::types::Json<serde_json::Value> = row.get("metadata");
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
