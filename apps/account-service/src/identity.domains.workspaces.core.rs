use chrono::{Duration, Utc};
use nvbes_product_account::cloud_boundary::{CreateWorkspaceCommand, WorkspacePolicyCommand};
use uuid::Uuid;

use super::db;
use super::types::*;
use super::validation::*;
use crate::{
    domains::{auth::types::AuthContext, cloud::workspace_port},
    http::error::AppError,
};
use sqlx::PgPool;

pub async fn list_workspaces(
    _db: &PgPool,
    auth: &AuthContext,
) -> Result<WorkspaceListResponse, AppError> {
    let mut workspaces = Vec::new();
    for workspace in workspace_port::list_workspaces(None, auth.user_id).await? {
        let role = active_member_role(
            Some(workspace.tenant_id),
            workspace.workspace_id,
            auth.user_id,
        )
        .await?;
        workspaces.push(db::workspace_response_from_cloud(workspace, &role).workspace);
    }

    Ok(WorkspaceListResponse { workspaces })
}

pub async fn create_workspace(
    db: &PgPool,
    auth: &AuthContext,
    input: CreateWorkspaceInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<WorkspaceResponse, AppError> {
    let name = validate_workspace_name(&input.name)?;
    let workspace_type = parse_workspace_type(input.workspace_type.as_deref())?;
    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::forbidden("tenant_required", "Tenant context is required."))?;

    let region = input.region.as_deref().unwrap_or("eu");
    let jurisdiction = input.jurisdiction.as_deref().unwrap_or("gdpr");

    let now = Utc::now();
    let trial_ends_at = now + Duration::days(14);

    let mut tx = db.begin().await?;

    let plan_code = "trial".to_string();
    let max_share_link_ttl_days = 7;

    let workspace_id = Uuid::new_v4();
    crate::domains::cloud::workspace_port::create_workspace_tx(
        &mut tx,
        &CreateWorkspaceCommand {
            workspace_id,
            tenant_id,
            organization_id: None,
            owner_principal_id: auth.user_id,
            name: name.clone(),
            workspace_type: workspace_type.to_string(),
            plan_code: plan_code.clone(),
            trial_ends_at: Some(trial_ends_at),
            data_region: region.to_string(),
            jurisdiction: jurisdiction.to_string(),
            owner_role: "owner".to_string(),
            membership_source: "manual".to_string(),
            created_at: now,
            policy: WorkspacePolicyCommand {
                member_can_create_share_links: false,
                require_admin_approval_for_member_share: true,
                default_share_link_ttl_days: 7,
                max_share_link_ttl_days,
                required_acr: None,
                mfa_policy: None,
            },
        },
    )
    .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id,
            workspace_id: Some(workspace_id),
            actor_principal_id: Some(auth.user_id),
            action: "workspace.created",
            target_type: "workspace",
            target_id: Some(workspace_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "workspace_type": workspace_type,
                "plan_code": "trial",
                "trial_days": 14,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    let workspace =
        workspace_port::get_workspace(Some(tenant_id), workspace_id, auth.user_id).await?;
    Ok(db::workspace_response_from_cloud(workspace, "owner"))
}

async fn active_member_role(
    tenant_id: Option<Uuid>,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<String, AppError> {
    workspace_port::list_workspace_members(tenant_id, workspace_id, actor_principal_id)
        .await?
        .into_iter()
        .find(|member| member.principal_id == actor_principal_id && member.active)
        .map(|member| member.role)
        .ok_or_else(|| {
            AppError::internal(
                "cloud_grpc_invalid_response",
                "Cloud returned a workspace without the caller membership.",
            )
        })
}
