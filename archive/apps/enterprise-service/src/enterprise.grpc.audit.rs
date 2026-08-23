use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    service_status::{non_empty, parse_uuid, sql_status},
};

pub async fn record_developer_secret_revoked(
    db: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    request: enterprise::RecordDeveloperSecretRevokedRequest,
) -> Result<enterprise::EnterpriseAuditRecord, Status> {
    let version_id = parse_uuid(&request.version_id, "version_id")?;
    let client_id = non_empty(request.client_id, "client_id")?;
    let row = sqlx::query_as::<_, AuditRecordRow>(
        r#"
        INSERT INTO audit_events (
          tenant_id, actor_principal_id, action, target_type, target_id,
          metadata, event_hash, created_at
        )
        VALUES (
          $1, $2, 'enterprise.developer_secret.revoked',
          'developer_client_secret_version', $3, $4,
          gen_random_uuid()::text, NOW()
        )
        RETURNING id, action, created_at
        "#,
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(version_id)
    .bind(serde_json::json!({ "client_id": client_id }))
    .fetch_one(db)
    .await
    .map_err(sql_status)?;

    Ok(enterprise::EnterpriseAuditRecord {
        event_id: row.id.to_string(),
        action: row.action,
        created_at: row.created_at.to_rfc3339(),
    })
}

pub async fn list_audit_events(
    db: &PgPool,
    tenant_id: Uuid,
    scope: &str,
    organization_id: &str,
    limit: i32,
) -> Result<enterprise::ListAuditEventsResponse, Status> {
    let limit = limit.clamp(1, 100);
    let rows = match scope.trim() {
        "tenant" => sqlx::query_as::<_, AuditEventRow>(
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
        .bind(i64::from(limit))
        .fetch_all(db)
        .await
        .map_err(sql_status)?,
        "organization" => {
            let organization_id = parse_uuid(organization_id, "organization_id")?;
            sqlx::query_as::<_, AuditEventRow>(
                r#"
                SELECT ae.id, ae.action, ae.actor_principal_id, u.email AS actor_email,
                  ae.target_type, ae.target_id, ae.metadata, ae.created_at
                FROM audit_events ae
                LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
                WHERE ae.tenant_id = $1
                  AND ae.workspace_id IN (SELECT id FROM workspaces WHERE organization_id = $3)
                ORDER BY ae.created_at DESC, ae.id DESC
                LIMIT $2
                "#,
            )
            .bind(tenant_id)
            .bind(i64::from(limit))
            .bind(organization_id)
            .fetch_all(db)
            .await
            .map_err(sql_status)?
        }
        _ => {
            return Err(Status::invalid_argument(
                "scope must be tenant or organization",
            ));
        }
    };
    Ok(enterprise::ListAuditEventsResponse {
        events: rows.into_iter().map(audit_event_from_row).collect(),
    })
}

fn audit_event_from_row(row: AuditEventRow) -> enterprise::AuditEvent {
    enterprise::AuditEvent {
        event_id: row.id.to_string(),
        event_type: row.action,
        actor_id: row
            .actor_principal_id
            .map(|actor_id| actor_id.to_string())
            .unwrap_or_default(),
        actor_email: row.actor_email.unwrap_or_default(),
        target_type: row.target_type.unwrap_or_default(),
        target_id: row
            .target_id
            .map(|target_id| target_id.to_string())
            .unwrap_or_default(),
        metadata_json: row.metadata.to_string(),
        created_at: row.created_at.to_rfc3339(),
    }
}

#[derive(sqlx::FromRow)]
struct AuditRecordRow {
    id: Uuid,
    action: String,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct AuditEventRow {
    id: Uuid,
    action: String,
    actor_principal_id: Option<Uuid>,
    actor_email: Option<String>,
    target_type: Option<String>,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
}

#[cfg(test)]
#[path = "enterprise.grpc.audit.contract_tests.rs"]
mod contract_tests;
