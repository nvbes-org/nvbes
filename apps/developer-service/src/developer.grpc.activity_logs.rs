use chrono::{DateTime, Utc};
use sqlx::{Row, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{parse_uuid, sql_status},
};

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 200;

pub async fn list_activity_logs(
    db: &sqlx::PgPool,
    request: developer::ListActivityLogsRequest,
) -> Result<developer::ListActivityLogsResponse, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let user_id = optional_uuid(&request.user_id, "user_id")?;
    let client_id = optional_text(&request.client_id);
    let event_type = optional_text(&request.event_type);
    let limit = if request.limit <= 0 {
        DEFAULT_LIMIT
    } else {
        request.limit.min(MAX_LIMIT)
    };

    let logs = sqlx::query(
        r#"
        SELECT id, event_type, user_id, client_id, tenant_id, created_at
        FROM (
          SELECT
            risk.id::text AS id,
            risk.event_type AS event_type,
            risk.principal_id AS user_id,
            risk.metadata->>'client_id' AS client_id,
            principal.tenant_id AS tenant_id,
            risk.created_at AS created_at
          FROM risk_events risk
          INNER JOIN principals principal ON principal.id = risk.principal_id
          WHERE principal.tenant_id = $1

          UNION ALL

          SELECT
            audit.id::text AS id,
            audit.action AS event_type,
            audit.actor_principal_id AS user_id,
            audit.metadata->>'client_id' AS client_id,
            audit.tenant_id AS tenant_id,
            audit.created_at AS created_at
          FROM audit_events audit
          WHERE audit.tenant_id = $1
        ) logs
        WHERE ($2::uuid IS NULL OR user_id = $2)
          AND ($3::text IS NULL OR client_id = $3)
          AND ($4::text IS NULL OR event_type = $4)
        ORDER BY created_at DESC
        LIMIT $5
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(client_id)
    .bind(event_type)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(activity_log_from_row)
    .collect();

    Ok(developer::ListActivityLogsResponse { logs })
}

fn optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, Status> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn activity_log_from_row(row: PgRow) -> developer::ActivityLogEntry {
    developer::ActivityLogEntry {
        id: row.get("id"),
        event_type: row.get("event_type"),
        user_id: row
            .get::<Option<Uuid>, _>("user_id")
            .map(|id| id.to_string())
            .unwrap_or_default(),
        client_id: row
            .get::<Option<String>, _>("client_id")
            .unwrap_or_default(),
        tenant_id: row
            .get::<Option<Uuid>, _>("tenant_id")
            .map(|id| id.to_string())
            .unwrap_or_default(),
        created_at: row.get::<DateTime<Utc>, _>("created_at").to_rfc3339(),
    }
}

#[cfg(test)]
#[path = "developer.grpc.activity_logs.contract_tests.rs"]
mod contract_tests;
