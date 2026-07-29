use chrono::{DateTime, Utc};
use sqlx::{Row, postgres::PgRow};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::{AccountCloudMock, cloud};

const WORKSPACE_SELECT: &str = r#"
    SELECT
      w.id,
      w.tenant_id,
      w.organization_id,
      w.name,
      w.workspace_type::text AS workspace_type,
      w.plan_code,
      w.trial_ends_at,
      w.data_region::text AS data_region,
      w.jurisdiction::text AS jurisdiction,
      w.status::text AS status,
      w.created_at,
      w.updated_at,
      p.member_can_create_share_links,
      p.require_admin_approval_for_member_share,
      p.default_share_link_ttl_days,
      p.max_share_link_ttl_days,
      p.required_acr::text AS required_acr,
      p.mfa_policy
    FROM workspaces w
    JOIN workspace_policies p ON p.workspace_id = w.id
"#;

impl AccountCloudMock {
    pub(super) async fn get_seeded_workspace(
        &self,
        request: Request<cloud::GetWorkspaceRequest>,
    ) -> Result<Response<cloud::Workspace>, Status> {
        let request = request.into_inner();
        let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
        let query = format!("{WORKSPACE_SELECT} WHERE w.id = $1");
        let row = sqlx::query(&query)
            .bind(workspace_id)
            .fetch_optional(&self.db)
            .await
            .map_err(|error| database_error("workspace lookup", error))?
            .ok_or_else(|| Status::not_found("seeded workspace was not found"))?;

        ensure_request_tenant(request.context, row.get("tenant_id"))?;
        Ok(Response::new(workspace_from_row(&row)?))
    }

    pub(super) async fn list_seeded_workspaces(
        &self,
        request: Request<cloud::ListWorkspacesRequest>,
    ) -> Result<Response<cloud::ListWorkspacesResponse>, Status> {
        let request = request.into_inner();
        let context = request
            .context
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("request context is required"))?;
        let actor_principal_id = parse_uuid(&context.actor_principal_id, "actor_principal_id")?;
        let context_tenant_id = context
            .tenant
            .as_ref()
            .map(|tenant| optional_uuid(&tenant.tenant_id, "tenant_id"))
            .transpose()?
            .flatten();

        let rows = if request.include_tenant_scope {
            let tenant_id = optional_uuid(&request.tenant_id, "tenant_id")?
                .or(context_tenant_id)
                .ok_or_else(|| Status::invalid_argument("tenant_id is required"))?;
            let organization_id = optional_uuid(&request.organization_id, "organization_id")?;
            let query = format!(
                "{WORKSPACE_SELECT}
                 WHERE w.tenant_id = $1
                   AND ($2::uuid IS NULL OR w.organization_id = $2)
                 ORDER BY w.created_at, w.id"
            );
            sqlx::query(&query)
                .bind(tenant_id)
                .bind(organization_id)
                .fetch_all(&self.db)
                .await
        } else {
            let query = format!(
                "{WORKSPACE_SELECT}
                 JOIN workspace_memberships wm ON wm.workspace_id = w.id
                 WHERE wm.principal_id = $1
                   AND wm.status = 'active'
                   AND ($2::uuid IS NULL OR w.tenant_id = $2)
                 ORDER BY w.created_at, w.id"
            );
            sqlx::query(&query)
                .bind(actor_principal_id)
                .bind(context_tenant_id)
                .fetch_all(&self.db)
                .await
        }
        .map_err(|error| database_error("workspace list", error))?;

        let workspaces = rows
            .iter()
            .map(workspace_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Response::new(cloud::ListWorkspacesResponse {
            workspaces,
            page: None,
        }))
    }

    pub(super) async fn list_seeded_workspace_members(
        &self,
        request: Request<cloud::ListWorkspaceMembersRequest>,
    ) -> Result<Response<cloud::ListWorkspaceMembersResponse>, Status> {
        let request = request.into_inner();
        let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
        let tenant_id =
            sqlx::query_scalar::<_, Uuid>("SELECT tenant_id FROM workspaces WHERE id = $1")
                .bind(workspace_id)
                .fetch_optional(&self.db)
                .await
                .map_err(|error| database_error("workspace tenant lookup", error))?
                .ok_or_else(|| Status::not_found("seeded workspace was not found"))?;
        ensure_request_tenant(request.context, tenant_id)?;

        let rows = sqlx::query(
            r#"
            SELECT
              principal_id,
              role::text AS role,
              status::text AS status,
              source::text AS source,
              created_at,
              updated_at
            FROM workspace_memberships
            WHERE workspace_id = $1
            ORDER BY principal_id
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.db)
        .await
        .map_err(|error| database_error("workspace membership lookup", error))?;

        let members = rows
            .into_iter()
            .map(|row| workspace_member_from_row(workspace_id, &row))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Response::new(cloud::ListWorkspaceMembersResponse {
            members,
            page: None,
        }))
    }
}

fn workspace_from_row(row: &PgRow) -> Result<cloud::Workspace, Status> {
    let created_at: DateTime<Utc> = row.get("created_at");
    let updated_at: DateTime<Utc> = row.get("updated_at");
    let trial_ends_at: Option<DateTime<Utc>> = row.get("trial_ends_at");
    let status: String = row.get("status");

    Ok(cloud::Workspace {
        workspace_id: row.get::<Uuid, _>("id").to_string(),
        name: row.get("name"),
        region_id: row.get("data_region"),
        data_residency: row.get("jurisdiction"),
        status: workspace_status(&status)?,
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        organization_id: row
            .get::<Option<Uuid>, _>("organization_id")
            .map(|value| value.to_string())
            .unwrap_or_default(),
        workspace_type: row.get("workspace_type"),
        plan_code: row.get("plan_code"),
        policy: Some(cloud::WorkspacePolicy {
            member_can_create_share_links: row.get("member_can_create_share_links"),
            require_admin_approval_for_member_share: row
                .get("require_admin_approval_for_member_share"),
            default_share_link_ttl_days: row.get("default_share_link_ttl_days"),
            max_share_link_ttl_days: row.get("max_share_link_ttl_days"),
            required_acr: row.get("required_acr"),
            mfa_policy: row.get("mfa_policy"),
        }),
        owner_principal_id: String::new(),
        trial_ends_at: trial_ends_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_default(),
        created_at: created_at.to_rfc3339(),
        updated_at: updated_at.to_rfc3339(),
    })
}

fn workspace_member_from_row(
    workspace_id: Uuid,
    row: &PgRow,
) -> Result<cloud::WorkspaceMember, Status> {
    let status: String = row.get("status");
    let created_at: DateTime<Utc> = row.get("created_at");
    let updated_at: DateTime<Utc> = row.get("updated_at");
    Ok(cloud::WorkspaceMember {
        workspace_id: workspace_id.to_string(),
        principal_id: row.get::<Uuid, _>("principal_id").to_string(),
        role: row.get("role"),
        status: workspace_member_status(&status)?,
        joined_at: created_at.to_rfc3339(),
        source: row.get("source"),
        updated_at: updated_at.to_rfc3339(),
    })
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(value)
        .map_err(|_| Status::invalid_argument(format!("{field} must be a valid UUID")))
}

fn optional_uuid(value: &str, field: &str) -> Result<Option<Uuid>, Status> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    parse_uuid(value, field).map(Some)
}

fn ensure_request_tenant(
    context: Option<crate::grpc_pb::nvbes::platform::v1::RequestContext>,
    actual_tenant_id: Uuid,
) -> Result<(), Status> {
    let Some(requested_tenant_id) = context
        .and_then(|context| context.tenant)
        .map(|tenant| optional_uuid(&tenant.tenant_id, "tenant_id"))
        .transpose()?
        .flatten()
    else {
        return Ok(());
    };
    if requested_tenant_id != actual_tenant_id {
        return Err(Status::permission_denied(
            "request tenant does not own the seeded workspace",
        ));
    }
    Ok(())
}

fn workspace_status(value: &str) -> Result<i32, Status> {
    match value {
        "active" => Ok(cloud::WorkspaceStatus::Active as i32),
        "suspended" => Ok(cloud::WorkspaceStatus::Suspended as i32),
        "deleted" => Ok(cloud::WorkspaceStatus::Deleting as i32),
        _ => Err(Status::internal(format!(
            "seeded workspace has unsupported status {value}"
        ))),
    }
}

fn workspace_member_status(value: &str) -> Result<i32, Status> {
    match value {
        "active" => Ok(cloud::WorkspaceMemberStatus::Active as i32),
        "suspended" => Ok(cloud::WorkspaceMemberStatus::Suspended as i32),
        "removed" => Ok(cloud::WorkspaceMemberStatus::Removed as i32),
        _ => Err(Status::internal(format!(
            "seeded workspace member has unsupported status {value}"
        ))),
    }
}

fn database_error(operation: &str, error: sqlx::Error) -> Status {
    Status::internal(format!(
        "Account Cloud test server {operation} failed: {error}"
    ))
}
