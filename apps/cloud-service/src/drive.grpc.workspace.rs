use chrono::{Duration, Utc};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::{cloud::v1 as cloud, platform::v1},
    service_status::{
        non_empty, optional_datetime, optional_string, optional_uuid, parse_uuid, sql_status,
        validate_context,
    },
    service_workspace_persistence::{
        default_policy, ensure_context_workspace, fetch_member, fetch_plan, fetch_workspace,
        insert_policy, request_tenant_id, update_membership, upsert_membership, workspace_select,
    },
    service_workspace_principals::ensure_principal,
    service_workspace_rows::{WorkspaceInvitationRow, WorkspaceMemberRow, WorkspaceRow},
};

pub use crate::grpc::service_workspace_invitations::{
    get_invitation_by_token_hash, list_invitations,
};

pub async fn create_workspace(
    db: &sqlx::PgPool,
    request: cloud::CreateWorkspaceRequest,
) -> Result<cloud::Workspace, Status> {
    let context = validate_context(request.context.as_ref(), None)?;
    let workspace_id =
        optional_uuid(&request.workspace_id, "workspace_id")?.unwrap_or_else(Uuid::new_v4);
    ensure_context_workspace(context, workspace_id)?;

    let tenant_id = request_tenant_id(&request.tenant_id, context)?;
    let organization_id = optional_uuid(&request.organization_id, "organization_id")?;
    let owner_principal_id = optional_uuid(&request.owner_principal_id, "owner_principal_id")?
        .unwrap_or(parse_uuid(
            &context.actor_principal_id,
            "actor_principal_id",
        )?);
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

    let mut tx = db.begin().await.map_err(sql_status)?;
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

    tx.commit().await.map_err(sql_status)?;
    fetch_workspace(db, workspace_id).await
}

pub async fn get_workspace(
    db: &sqlx::PgPool,
    request: cloud::GetWorkspaceRequest,
) -> Result<cloud::Workspace, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    fetch_workspace(db, workspace_id).await
}

pub async fn update_workspace(
    db: &sqlx::PgPool,
    request: cloud::UpdateWorkspaceRequest,
) -> Result<cloud::Workspace, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let mut tx = db.begin().await.map_err(sql_status)?;

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

    tx.commit().await.map_err(sql_status)?;
    fetch_workspace(db, workspace_id).await
}

pub async fn delete_workspace(
    db: &sqlx::PgPool,
    request: cloud::DeleteWorkspaceRequest,
) -> Result<cloud::WorkspaceDeletion, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let result = sqlx::query("UPDATE workspaces SET deleted_at = NOW() WHERE id = $1")
        .bind(workspace_id)
        .execute(db)
        .await
        .map_err(sql_status)?;
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
    let context = validate_context(request.context.as_ref(), None)?;
    let actor_principal_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
    let tenant_id = context
        .tenant
        .as_ref()
        .and_then(|tenant| optional_uuid(&tenant.tenant_id, "tenant_id").ok().flatten());
    let workspaces = if request.include_tenant_scope {
        let scoped_tenant_id = request_tenant_id(&request.tenant_id, context)?;
        let organization_id = optional_uuid(&request.organization_id, "organization_id")?;
        let query = workspace_select(
            "WHERE w.deleted_at IS NULL
               AND w.tenant_id = $1
               AND ($2::uuid IS NULL OR w.organization_id = $2)
             ORDER BY w.created_at ASC",
        );
        sqlx::query_as::<_, WorkspaceRow>(&query)
            .bind(scoped_tenant_id)
            .bind(organization_id)
            .fetch_all(db)
            .await
            .map_err(sql_status)?
    } else {
        let query = workspace_select(
            "INNER JOIN workspace_memberships wm ON wm.workspace_id = w.id
             WHERE wm.user_id = $1 AND w.deleted_at IS NULL
               AND ($2::uuid IS NULL OR w.tenant_id = $2)
             ORDER BY w.created_at ASC",
        );
        sqlx::query_as::<_, WorkspaceRow>(&query)
            .bind(actor_principal_id)
            .bind(tenant_id)
            .fetch_all(db)
            .await
            .map_err(sql_status)?
    }
    .into_iter()
    .map(WorkspaceRow::into_proto)
    .collect();

    Ok(cloud::ListWorkspacesResponse {
        workspaces,
        page: Some(v1::CursorPageResponse {
            next_cursor: String::new(),
            has_more: false,
        }),
    })
}

pub async fn add_member(
    db: &sqlx::PgPool,
    request: cloud::AddWorkspaceMemberRequest,
) -> Result<cloud::WorkspaceMember, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    let role = non_empty(request.role, "role")?;
    let source = optional_string(request.source).unwrap_or_else(|| "manual".to_string());
    let mut tx = db.begin().await.map_err(sql_status)?;
    ensure_principal(
        &mut tx,
        principal_id,
        request.principal.as_ref(),
        "principal_id",
    )
    .await?;
    upsert_membership(
        &mut tx,
        workspace_id,
        principal_id,
        &role,
        "active",
        &source,
    )
    .await?;
    tx.commit().await.map_err(sql_status)?;
    fetch_member(db, workspace_id, principal_id).await
}

pub async fn update_member(
    db: &sqlx::PgPool,
    request: cloud::UpdateWorkspaceMemberRequest,
) -> Result<cloud::WorkspaceMember, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    let role = optional_string(request.role);
    let status = optional_string(request.status);
    let source = optional_string(request.source);
    let mut tx = db.begin().await.map_err(sql_status)?;
    if request.principal.is_some() {
        ensure_principal(
            &mut tx,
            principal_id,
            request.principal.as_ref(),
            "principal_id",
        )
        .await?;
    }
    update_membership(&mut tx, workspace_id, principal_id, role, status, source).await?;
    tx.commit().await.map_err(sql_status)?;
    fetch_member(db, workspace_id, principal_id).await
}

pub async fn remove_member(
    db: &sqlx::PgPool,
    request: cloud::RemoveWorkspaceMemberRequest,
) -> Result<cloud::WorkspaceMembershipRemoval, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    let result = sqlx::query(
        "UPDATE workspace_memberships SET status = 'removed', updated_at = NOW() WHERE workspace_id = $1 AND user_id = $2",
    )
    .bind(workspace_id)
    .bind(principal_id)
    .execute(db)
    .await
    .map_err(sql_status)?;
    Ok(cloud::WorkspaceMembershipRemoval {
        workspace_id: workspace_id.to_string(),
        principal_id: principal_id.to_string(),
        removed: result.rows_affected() > 0,
    })
}

pub async fn list_members(
    db: &sqlx::PgPool,
    request: cloud::ListWorkspaceMembersRequest,
) -> Result<cloud::ListWorkspaceMembersResponse, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let members = sqlx::query_as::<_, WorkspaceMemberRow>(
        r#"
        SELECT workspace_id, user_id AS principal_id, role::text AS role, status::text AS status, source, created_at AS joined_at, updated_at
        FROM workspace_memberships
        WHERE workspace_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(WorkspaceMemberRow::into_proto)
    .collect();
    Ok(cloud::ListWorkspaceMembersResponse {
        members,
        page: Some(v1::CursorPageResponse {
            next_cursor: String::new(),
            has_more: false,
        }),
    })
}

pub async fn create_invitation(
    db: &sqlx::PgPool,
    request: cloud::CreateWorkspaceInvitationRequest,
) -> Result<cloud::WorkspaceInvitation, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let invited_by = parse_uuid(&request.invited_by_principal_id, "invited_by_principal_id")?;
    let email = non_empty(request.email, "email")?.to_lowercase();
    let role = non_empty(request.role, "role")?;
    let token_hash = non_empty(request.token_hash, "token_hash")?;
    let expires_at = optional_datetime(&request.expires_at, "expires_at")?
        .unwrap_or_else(|| Utc::now() + Duration::days(7));
    let mut tx = db.begin().await.map_err(sql_status)?;
    ensure_principal(
        &mut tx,
        invited_by,
        request.invited_by_principal.as_ref(),
        "invited_by_principal_id",
    )
    .await?;
    let invitation = sqlx::query_as::<_, WorkspaceInvitationRow>(
        r#"
        INSERT INTO workspace_invitations (workspace_id, email, role, invited_by, token_hash, expires_at)
        VALUES ($1, $2, $3::workspace_member_role, $4, $5, $6)
        RETURNING id AS invitation_id, workspace_id, email, role::text AS role, status::text AS status, expires_at, accepted_at, revoked_at, created_at
        "#,
    )
    .bind(workspace_id)
    .bind(email)
    .bind(role)
    .bind(invited_by)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(sql_status)?;
    tx.commit().await.map_err(sql_status)?;
    Ok(invitation.into_proto())
}

pub async fn accept_invitation(
    db: &sqlx::PgPool,
    request: cloud::AcceptWorkspaceInvitationRequest,
) -> Result<cloud::WorkspaceInvitationAcceptance, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let invitation_id = parse_uuid(&request.invitation_id, "invitation_id")?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    let role = non_empty(request.role, "role")?;
    let accepted_at =
        optional_datetime(&request.accepted_at, "accepted_at")?.unwrap_or_else(Utc::now);
    let mut tx = db.begin().await.map_err(sql_status)?;
    ensure_principal(
        &mut tx,
        principal_id,
        request.principal.as_ref(),
        "principal_id",
    )
    .await?;
    upsert_membership(
        &mut tx,
        workspace_id,
        principal_id,
        &role,
        "active",
        "invitation",
    )
    .await?;
    sqlx::query(
        "UPDATE workspace_invitations SET status = 'accepted', accepted_at = $2, updated_at = $2 WHERE id = $1 AND workspace_id = $3",
    )
    .bind(invitation_id)
    .bind(accepted_at)
    .bind(workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(sql_status)?;
    tx.commit().await.map_err(sql_status)?;
    Ok(cloud::WorkspaceInvitationAcceptance {
        invitation_id: invitation_id.to_string(),
        workspace_id: workspace_id.to_string(),
        principal_id: principal_id.to_string(),
        accepted: true,
    })
}
