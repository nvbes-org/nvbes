use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone)]
pub struct AuthAuditInput<'a> {
    pub principal_id: Uuid,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub metadata: Value,
}

pub async fn record_auth_event(db: &PgPool, input: AuthAuditInput<'_>) -> Result<(), AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          p.tenant_id,
          (
            SELECT wm.workspace_id
            FROM workspace_memberships wm
            WHERE wm.principal_id = p.id
              AND wm.status = 'active'
            ORDER BY
              CASE wm.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END,
              wm.created_at ASC
            LIMIT 1
          ) AS workspace_id
        FROM principals p
        WHERE p.id = $1
        LIMIT 1
        "#,
    )
    .bind(input.principal_id)
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(());
    };
    let Some(tenant_id) = row.get::<Option<Uuid>, _>("tenant_id") else {
        return Ok(());
    };

    crate::domains::audit::record_event(
        db,
        crate::domains::audit::AuditRecordInput {
            tenant_id,
            workspace_id: row.get("workspace_id"),
            actor_principal_id: Some(input.principal_id),
            action: input.action,
            target_type: input.target_type,
            target_id: input.target_id,
            ip: input.ip,
            user_agent: input.user_agent,
            metadata: input.metadata,
        },
    )
    .await?;

    Ok(())
}
