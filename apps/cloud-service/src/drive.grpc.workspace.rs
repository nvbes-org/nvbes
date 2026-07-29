use chrono::Utc;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::{cloud::v1 as cloud, platform::v1},
    service_status::{
        non_empty, optional_datetime, optional_string, optional_uuid, parse_uuid, sql_status,
    },
    service_workspace_context::{begin_request_transaction, set_request_context},
    service_workspace_persistence::{
        default_policy, fetch_plan, fetch_workspace, insert_policy, request_tenant_id,
        upsert_membership,
    },
    service_workspace_principals::ensure_principal,
};

pub use crate::grpc::service_workspace_invitations::{
    accept_invitation, create_invitation, get_invitation_by_token_hash, list_invitations,
};
pub use crate::grpc::service_workspace_members::{
    add_member, list_members, remove_member, update_member,
};

pub async fn create_workspace(
    db: &sqlx::PgPool,
    request: cloud::CreateWorkspaceRequest,
) -> Result<cloud::Workspace, Status> {
    let workspace_id =
        optional_uuid(&request.workspace_id, "workspace_id")?.unwrap_or_else(Uuid::new_v4);
    let organization_id = optional_uuid(&request.organization_id, "organization_id")?;
    let name = non_empty(request.name, "name")?;
    let workspace_type =
        optional_string(request.workspace_type).unwrap_or_else(|| "personal".to_string());
    let plan_code = optional_string(request.plan_code).unwrap_or_else(|| "trial".to_string());
    let data_region = optional_string(request.region_id)
        .or_else(|| optional_string(request.data_residency.clone()))
        .unwrap_or_else(|| "eu".to_string());
    let jurisdiction =
        optional_string(request.legal_jurisdiction).unwrap_or_else(|| "gdpr".to_string());
    let owner_role = optional_string(request.owner_role).unwrap_or_else(|| "owner".to_string());
    let membership_source =
        optional_string(request.membership_source).unwrap_or_else(|| "manual".to_string());
    let trial_ends_at = optional_datetime(&request.trial_ends_at, "trial_ends_at")?;
    let created_at = optional_datetime(&request.created_at, "created_at")?.unwrap_or_else(Utc::now);

    let (mut tx, context) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
    let tenant_id = request_tenant_id(&request.tenant_id, context)?;
    let owner_principal_id = optional_uuid(&request.owner_principal_id, "owner_principal_id")?
        .unwrap_or(context.actor_principal_id);
    ensure_principal(
        &mut tx,
        owner_principal_id,
        request.owner_principal.as_ref(),
        "owner_principal_id",
    )
    .await?;

    let plan = fetch_plan(&mut tx, &plan_code).await?;
    let policy = request.policy.unwrap_or_else(|| default_policy(&plan));
    let trial_started_at = trial_ends_at.as_ref().map(|_| created_at);

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id,
          tenant_id,
          organization_id,
          workspace_type,
          name,
          owner_user_id,
          owner_principal_id,
          plan_id,
          trial_started_at,
          trial_ends_at,
          data_region,
          jurisdiction,
          created_at,
          updated_at
        )
        VALUES ($1, $2, $3, $4::workspace_type, $5, $6, $6, $7, $8, $9, $10::data_region, $11::legal_jurisdiction, $12, $12)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(&workspace_type)
    .bind(&name)
    .bind(owner_principal_id)
    .bind(plan.id)
    .bind(trial_started_at)
    .bind(trial_ends_at)
    .bind(&data_region)
    .bind(&jurisdiction)
    .bind(created_at)
    .execute(&mut *tx)
    .await
    .map_err(sql_status)?;

    insert_policy(&mut tx, workspace_id, &policy, &plan, created_at).await?;
    upsert_membership(
        &mut tx,
        workspace_id,
        owner_principal_id,
        &owner_role,
        "active",
        &membership_source,
    )
    .await?;

    sqlx::query("INSERT INTO quota_usage (workspace_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(workspace_id)
        .execute(&mut *tx)
        .await
        .map_err(sql_status)?;

    let workspace = fetch_workspace(&mut tx, workspace_id).await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(workspace)
}

pub async fn get_workspace(
    db: &sqlx::PgPool,
    request: cloud::GetWorkspaceRequest,
) -> Result<cloud::Workspace, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
    let workspace = fetch_workspace(&mut tx, workspace_id).await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(workspace)
}

pub async fn update_workspace(
    db: &sqlx::PgPool,
    request: cloud::UpdateWorkspaceRequest,
) -> Result<cloud::Workspace, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;

    if let Some(name) = optional_string(request.name) {
        sqlx::query("UPDATE workspaces SET name = $2, updated_at = NOW() WHERE id = $1")
            .bind(workspace_id)
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(sql_status)?;
    }

    if let Some(policy) = request.policy {
        sqlx::query(
            r#"
            UPDATE workspace_policies
            SET member_can_create_share_links = $2,
                require_admin_approval_for_member_share = $3,
                default_share_link_ttl_days = $4,
                max_share_link_ttl_days = $5,
                required_acr = COALESCE(NULLIF($6, ''), required_acr),
                mfa_policy = COALESCE(NULLIF($7, ''), mfa_policy),
                updated_at = NOW()
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .bind(policy.member_can_create_share_links)
        .bind(policy.require_admin_approval_for_member_share)
        .bind(policy.default_share_link_ttl_days)
        .bind(policy.max_share_link_ttl_days)
        .bind(policy.required_acr)
        .bind(policy.mfa_policy)
        .execute(&mut *tx)
        .await
        .map_err(sql_status)?;
    }

    let workspace = fetch_workspace(&mut tx, workspace_id).await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(workspace)
}

pub async fn delete_workspace(
    db: &sqlx::PgPool,
    request: cloud::DeleteWorkspaceRequest,
) -> Result<cloud::WorkspaceDeletion, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
    let result = sqlx::query("UPDATE workspaces SET deleted_at = NOW() WHERE id = $1")
        .bind(workspace_id)
        .execute(&mut *tx)
        .await
        .map_err(sql_status)?;
    tx.commit().await.map_err(sql_status)?;
    Ok(cloud::WorkspaceDeletion {
        workspace_id: workspace_id.to_string(),
        status: if result.rows_affected() == 0 {
            cloud::WorkspaceDeletionStatus::Unspecified as i32
        } else {
            cloud::WorkspaceDeletionStatus::Accepted as i32
        },
    })
}

pub async fn list_workspaces(
    db: &sqlx::PgPool,
    request: cloud::ListWorkspacesRequest,
) -> Result<cloud::ListWorkspacesResponse, Status> {
    let (mut tx, context) = begin_request_transaction(db, request.context.as_ref(), None).await?;
    let workspace_rows = if request.include_tenant_scope {
        let scoped_tenant_id = request_tenant_id(&request.tenant_id, context)?;
        let organization_id = optional_uuid(&request.organization_id, "organization_id")?;
        sqlx::query_as::<_, (Uuid, Option<Uuid>)>(
            "SELECT w.id, w.tenant_id
             FROM workspaces w
             WHERE w.deleted_at IS NULL
               AND w.tenant_id = $1
               AND ($2::uuid IS NULL OR w.organization_id = $2)
             ORDER BY w.created_at ASC",
        )
        .bind(scoped_tenant_id)
        .bind(organization_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(sql_status)?
    } else {
        sqlx::query_as::<_, (Uuid, Option<Uuid>)>(
            "SELECT w.id, w.tenant_id
             FROM workspaces w
             INNER JOIN workspace_memberships wm ON wm.workspace_id = w.id
             WHERE wm.user_id = $1 AND w.deleted_at IS NULL
               AND ($2::uuid IS NULL OR w.tenant_id = $2)
             ORDER BY w.created_at ASC",
        )
        .bind(context.actor_principal_id)
        .bind(context.tenant_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(sql_status)?
    };
    let mut workspaces = Vec::with_capacity(workspace_rows.len());
    for (workspace_id, tenant_id) in workspace_rows {
        let mut workspace_context = context;
        workspace_context.workspace_id = Some(workspace_id);
        workspace_context.tenant_id = tenant_id.or(context.tenant_id);
        set_request_context(&mut tx, workspace_context).await?;
        workspaces.push(fetch_workspace(&mut tx, workspace_id).await?);
    }

    tx.commit().await.map_err(sql_status)?;
    Ok(cloud::ListWorkspacesResponse {
        workspaces,
        page: Some(v1::CursorPageResponse {
            next_cursor: String::new(),
            has_more: false,
        }),
    })
}
