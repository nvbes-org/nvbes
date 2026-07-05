use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::authz::AdminScope;
use crate::http::error::AppError;

use super::super::records::AuditEventRow;

pub async fn list_audit_events(
    db: &PgPool,
    tenant_id: Uuid,
    scope: AdminScope,
    limit: i64,
) -> Result<Vec<AuditEventRow>, AppError> {
    match scope {
        AdminScope::Tenant => Ok(sqlx::query_as::<_, AuditEventRow>(
            r#"
                SELECT ae.id, ae.action AS event_type, ae.actor_principal_id AS actor_id,
                  u.email AS actor_email, ae.target_type, ae.target_id, ae.metadata, ae.created_at
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
        .await?),
        AdminScope::Organization(org_id) => Ok(sqlx::query_as::<_, AuditEventRow>(
            r#"
                SELECT ae.id, ae.action AS event_type, ae.actor_principal_id AS actor_id,
                  u.email AS actor_email, ae.target_type, ae.target_id, ae.metadata, ae.created_at
                FROM audit_events ae
                LEFT JOIN users u ON u.principal_id = ae.actor_principal_id
                WHERE ae.tenant_id = $1
                  AND ae.workspace_id IN (SELECT id FROM workspaces WHERE organization_id = $3)
                ORDER BY ae.created_at DESC, ae.id DESC
                LIMIT $2
                "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(org_id)
        .fetch_all(db)
        .await?),
    }
}
