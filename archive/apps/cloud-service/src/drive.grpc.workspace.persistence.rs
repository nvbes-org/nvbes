use chrono::{DateTime, Utc};
use sqlx::FromRow;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::cloud::v1 as cloud,
    service_status::{optional_uuid, sql_status},
    service_workspace_context::ValidatedRequestContext,
    service_workspace_rows::{WorkspaceMemberRow, WorkspaceRow},
};

#[derive(FromRow)]
pub(crate) struct WorkspacePlan {
    pub id: Uuid,
    pub max_share_link_ttl_days: i32,
}

pub(crate) async fn fetch_plan(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    plan_code: &str,
) -> Result<WorkspacePlan, Status> {
    sqlx::query_as::<_, WorkspacePlan>(
        "SELECT id, max_share_link_ttl_days FROM plans WHERE code = $1 LIMIT 1",
    )
    .bind(plan_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::failed_precondition("cloud plan code is not configured"))
}

pub(crate) fn default_policy(plan: &WorkspacePlan) -> cloud::WorkspacePolicy {
    cloud::WorkspacePolicy {
        member_can_create_share_links: false,
        require_admin_approval_for_member_share: true,
        default_share_link_ttl_days: 7,
        max_share_link_ttl_days: plan.max_share_link_ttl_days,
        required_acr: "aal1".to_string(),
        mfa_policy: "optional".to_string(),
    }
}

pub(crate) async fn insert_policy(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    policy: &cloud::WorkspacePolicy,
    plan: &WorkspacePlan,
    created_at: DateTime<Utc>,
) -> Result<(), Status> {
    let max_ttl = if policy.max_share_link_ttl_days > 0 {
        policy.max_share_link_ttl_days
    } else {
        plan.max_share_link_ttl_days
    };
    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id,
          member_can_create_share_links,
          require_admin_approval_for_member_share,
          default_share_link_ttl_days,
          max_share_link_ttl_days,
          required_acr,
          mfa_policy,
          updated_at
        )
        VALUES ($1, $2, $3, $4, $5, COALESCE(NULLIF($6, ''), 'aal1'), COALESCE(NULLIF($7, ''), 'optional'), $8)
        "#,
    )
    .bind(workspace_id)
    .bind(policy.member_can_create_share_links)
    .bind(policy.require_admin_approval_for_member_share)
    .bind(policy.default_share_link_ttl_days.max(1))
    .bind(max_ttl)
    .bind(&policy.required_acr)
    .bind(&policy.mfa_policy)
    .bind(created_at)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(())
}

pub(crate) async fn upsert_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    principal_id: Uuid,
    role: &str,
    status: &str,
    source: &str,
) -> Result<(), Status> {
    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, user_id, role, status, source)
        VALUES ($1, $2, $3::workspace_member_role, $4::workspace_member_status, $5)
        ON CONFLICT (workspace_id, user_id)
        DO UPDATE SET role = EXCLUDED.role, status = EXCLUDED.status, source = EXCLUDED.source, updated_at = NOW()
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .bind(role)
    .bind(status)
    .bind(source)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    Ok(())
}

pub(crate) async fn update_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    principal_id: Uuid,
    role: Option<String>,
    status: Option<String>,
    source: Option<String>,
) -> Result<(), Status> {
    let result = sqlx::query(
        r#"
        UPDATE workspace_memberships
        SET role = COALESCE($3::workspace_member_role, role),
            status = COALESCE($4::workspace_member_status, status),
            source = COALESCE($5, source),
            updated_at = NOW()
        WHERE workspace_id = $1 AND user_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .bind(role)
    .bind(status)
    .bind(source)
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;
    if result.rows_affected() == 0 {
        Err(Status::not_found("cloud workspace member was not found"))
    } else {
        Ok(())
    }
}

pub(crate) async fn fetch_workspace(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<cloud::Workspace, Status> {
    let query = workspace_select("WHERE w.id = $1 AND w.deleted_at IS NULL");
    sqlx::query_as::<_, WorkspaceRow>(&query)
        .bind(workspace_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(sql_status)?
        .ok_or_else(|| Status::not_found("cloud workspace was not found"))
        .map(WorkspaceRow::into_proto)
}

pub(crate) async fn fetch_member(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    principal_id: Uuid,
) -> Result<cloud::WorkspaceMember, Status> {
    sqlx::query_as::<_, WorkspaceMemberRow>(
        r#"
        SELECT workspace_id, user_id AS principal_id, role::text AS role, status::text AS status,
               source::text AS source, created_at AS joined_at, updated_at
        FROM workspace_memberships
        WHERE workspace_id = $1 AND user_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("cloud workspace member was not found"))
    .map(WorkspaceMemberRow::into_proto)
}

pub(crate) fn workspace_select(filter: &str) -> String {
    format!(
        r#"
        SELECT
          w.id AS workspace_id,
          w.name,
          w.data_region::text AS data_region,
          w.jurisdiction::text AS jurisdiction,
          w.owner_principal_id,
          w.tenant_id,
          w.organization_id,
          w.workspace_type::text AS workspace_type,
          p.code AS plan_code,
          w.trial_ends_at,
          wp.member_can_create_share_links,
          wp.require_admin_approval_for_member_share,
          wp.default_share_link_ttl_days,
          wp.max_share_link_ttl_days,
          COALESCE(wp.required_acr, 'aal1') AS required_acr,
          COALESCE(wp.mfa_policy, 'optional') AS mfa_policy,
          w.created_at,
          w.updated_at
        FROM workspaces w
        INNER JOIN plans p ON p.id = w.plan_id
        INNER JOIN workspace_policies wp ON wp.workspace_id = w.id
        {filter}
        "#
    )
}

pub(crate) fn request_tenant_id(
    value: &str,
    context: ValidatedRequestContext,
) -> Result<Uuid, Status> {
    let requested = optional_uuid(value, "tenant_id")?;
    match (requested, context.tenant_id) {
        (Some(requested), Some(context)) if requested != context => Err(Status::permission_denied(
            "request tenant does not match context tenant",
        )),
        (Some(requested), _) => Ok(requested),
        (None, Some(context)) => Ok(context),
        (None, None) => Err(Status::invalid_argument("tenant_id is required")),
    }
}
