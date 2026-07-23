use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{cloud_boundary::workspace_port, http::error::AppError};

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
        SELECT p.tenant_id
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
    let workspace_id = audit_workspace_id(tenant_id, input.principal_id).await?;

    crate::domains::audit::record_event(
        db,
        crate::domains::audit::AuditRecordInput {
            tenant_id,
            workspace_id,
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

async fn audit_workspace_id(tenant_id: Uuid, principal_id: Uuid) -> Result<Option<Uuid>, AppError> {
    let mut selected = None;
    let mut selected_rank = i32::MAX;
    for workspace in workspace_port::list_workspaces(Some(tenant_id), principal_id).await? {
        let rank = workspace_port::list_workspace_members(
            Some(tenant_id),
            workspace.workspace_id,
            principal_id,
        )
        .await?
        .into_iter()
        .find(|member| member.principal_id == principal_id && member.active)
        .map(|member| audit_workspace_role_rank(&member.role))
        .unwrap_or(i32::MAX);
        if rank < selected_rank {
            selected_rank = rank;
            selected = Some(workspace.workspace_id);
        }
    }
    Ok(selected)
}

fn audit_workspace_role_rank(role: &str) -> i32 {
    match role {
        "owner" => 0,
        "admin" => 1,
        "member" => 2,
        _ => 3,
    }
}
