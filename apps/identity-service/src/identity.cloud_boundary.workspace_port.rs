use axum::http::StatusCode;
use sqlx::{Postgres, Row, Transaction};
use tonic::{Code, transport::Channel};
use uuid::Uuid;

use crate::{
    grpc_pb::nvbes::{
        cloud::v1::{
            GetWorkspaceRequest, ListWorkspaceMembersRequest, ListWorkspacesRequest,
            PrincipalProjection, Workspace, cloud_service_client::CloudServiceClient,
        },
        platform::v1::{RequestContext, TenantContext},
    },
    http::error::AppError,
};

#[path = "identity.cloud_boundary.workspace_port.invitations.rs"]
mod workspace_port_invitations;
#[path = "identity.cloud_boundary.workspace_port.mutations.rs"]
mod workspace_port_mutations;
#[path = "identity.cloud_boundary.workspace_port.parse.rs"]
mod workspace_port_parse;
#[path = "identity.cloud_boundary.workspace_port.types.rs"]
mod workspace_port_types;
#[path = "identity.cloud_boundary.workspace_port.workspaces.rs"]
mod workspace_port_workspaces;
pub use workspace_port_invitations::{
    get_workspace_invitation_by_token_hash, list_workspace_invitations,
};
pub use workspace_port_mutations::{
    accept_workspace_invitation_tx, create_workspace_invitation_tx, create_workspace_tx,
    update_workspace_membership_role_tx, update_workspace_membership_status_tx,
    update_workspace_settings_tx, upsert_workspace_membership_tx,
};
use workspace_port_parse::{
    optional_string, parse_datetime, parse_optional_datetime, parse_optional_uuid, parse_uuid,
    workspace_member_status,
};
pub use workspace_port_types::{CloudWorkspaceMemberSummary, CloudWorkspaceSummary};
pub use workspace_port_workspaces::list_tenant_workspaces;

const CLOUD_GRPC_ENDPOINT_ENV: &str = "NVBES_CLOUD_GRPC_ENDPOINT";

pub fn cloud_grpc_endpoint() -> anyhow::Result<String> {
    match std::env::var(CLOUD_GRPC_ENDPOINT_ENV) {
        Ok(endpoint) if !endpoint.trim().is_empty() => Ok(endpoint),
        Ok(_) => anyhow::bail!("{CLOUD_GRPC_ENDPOINT_ENV} must not be empty"),
        Err(std::env::VarError::NotPresent) => Ok("http://127.0.0.1:4003".to_string()),
        Err(error) => Err(anyhow::anyhow!(
            "{CLOUD_GRPC_ENDPOINT_ENV} could not be read: {error}"
        )),
    }
}

pub async fn get_workspace(
    tenant_id: Option<Uuid>,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<CloudWorkspaceSummary, AppError> {
    let mut client = cloud_client().await?;
    let workspace = client
        .get_workspace(GetWorkspaceRequest {
            context: Some(request_context(
                tenant_id,
                workspace_id,
                actor_principal_id,
                "",
            )),
            workspace_id: workspace_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    workspace_summary(workspace)
}

pub async fn list_workspaces(
    tenant_id: Option<Uuid>,
    actor_principal_id: Uuid,
) -> Result<Vec<CloudWorkspaceSummary>, AppError> {
    let mut client = cloud_client().await?;
    let response = client
        .list_workspaces(ListWorkspacesRequest {
            context: Some(request_context(
                tenant_id,
                Uuid::nil(),
                actor_principal_id,
                "",
            )),
            page: None,
            tenant_id: String::new(),
            organization_id: String::new(),
            include_tenant_scope: false,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    response
        .workspaces
        .into_iter()
        .map(workspace_summary)
        .collect()
}

pub async fn list_workspace_members(
    tenant_id: Option<Uuid>,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<Vec<CloudWorkspaceMemberSummary>, AppError> {
    let mut client = cloud_client().await?;
    let response = client
        .list_workspace_members(ListWorkspaceMembersRequest {
            context: Some(request_context(
                tenant_id,
                workspace_id,
                actor_principal_id,
                "",
            )),
            workspace_id: workspace_id.to_string(),
            page: None,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    response
        .members
        .into_iter()
        .map(|member| {
            let status = workspace_member_status(member.status);
            Ok(CloudWorkspaceMemberSummary {
                principal_id: parse_uuid(&member.principal_id, "principal_id")?,
                role: member.role,
                status: status.to_string(),
                active: status == "active",
                joined_at: parse_datetime(&member.joined_at, "joined_at")?,
                updated_at: parse_datetime(&member.updated_at, "updated_at")?,
            })
        })
        .collect()
}

async fn cloud_client() -> Result<CloudServiceClient<Channel>, AppError> {
    let endpoint = cloud_grpc_endpoint()
        .map_err(|error| AppError::internal("cloud_grpc_endpoint_invalid", error.to_string()))?;
    CloudServiceClient::connect(endpoint)
        .await
        .map_err(|error| AppError::internal("cloud_grpc_connect_failed", error.to_string()))
}

async fn principal_projection_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<PrincipalProjection, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          p.display_name,
          p.principal_kind::text AS principal_kind,
          u.email
        FROM principals p
        LEFT JOIN users u ON u.principal_id = p.id
        WHERE p.id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::not_found("principal_not_found", "Principal not found."))?;

    let email: Option<String> = row.get("email");
    Ok(PrincipalProjection {
        principal_id: principal_id.to_string(),
        email: email.unwrap_or_default(),
        display_name: row.get("display_name"),
        principal_kind: row.get("principal_kind"),
    })
}

fn request_context(
    tenant_id: Option<Uuid>,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
    data_region: &str,
) -> RequestContext {
    RequestContext {
        request_id: Uuid::new_v4().to_string(),
        correlation_id: Uuid::new_v4().to_string(),
        actor_principal_id: actor_principal_id.to_string(),
        tenant: Some(TenantContext {
            tenant_id: tenant_id.map(|id| id.to_string()).unwrap_or_default(),
            workspace_id: workspace_id.to_string(),
            region_id: data_region.to_string(),
            data_residency: data_region.to_string(),
        }),
    }
}

fn workspace_summary(workspace: Workspace) -> Result<CloudWorkspaceSummary, AppError> {
    let policy = workspace.policy.as_ref();
    Ok(CloudWorkspaceSummary {
        workspace_id: parse_uuid(&workspace.workspace_id, "workspace_id")?,
        name: workspace.name,
        tenant_id: parse_uuid(&workspace.tenant_id, "tenant_id")?,
        organization_id: parse_optional_uuid(&workspace.organization_id, "organization_id")?,
        workspace_type: workspace.workspace_type,
        plan_code: workspace.plan_code,
        data_region: optional_string(workspace.region_id),
        jurisdiction: optional_string(workspace.data_residency),
        member_can_create_share_links: policy
            .map(|policy| policy.member_can_create_share_links)
            .unwrap_or(false),
        require_admin_approval_for_member_share: policy
            .map(|policy| policy.require_admin_approval_for_member_share)
            .unwrap_or(true),
        default_share_link_ttl_days: policy
            .map(|policy| policy.default_share_link_ttl_days)
            .unwrap_or(7),
        max_share_link_ttl_days: policy
            .map(|policy| policy.max_share_link_ttl_days)
            .unwrap_or(7),
        required_acr: policy.and_then(|policy| optional_string(policy.required_acr.clone())),
        mfa_policy: policy.and_then(|policy| optional_string(policy.mfa_policy.clone())),
        owner_principal_id: parse_optional_uuid(
            &workspace.owner_principal_id,
            "owner_principal_id",
        )?,
        trial_ends_at: parse_optional_datetime(&workspace.trial_ends_at, "trial_ends_at")?,
        created_at: parse_datetime(&workspace.created_at, "created_at")?,
        updated_at: parse_datetime(&workspace.updated_at, "updated_at")?,
    })
}

fn grpc_error(error: tonic::Status) -> AppError {
    let status = match error.code() {
        Code::InvalidArgument => StatusCode::BAD_REQUEST,
        Code::Unauthenticated => StatusCode::UNAUTHORIZED,
        Code::PermissionDenied => StatusCode::FORBIDDEN,
        Code::NotFound => StatusCode::NOT_FOUND,
        Code::AlreadyExists | Code::FailedPrecondition => StatusCode::CONFLICT,
        Code::Unavailable | Code::DeadlineExceeded => StatusCode::BAD_GATEWAY,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    AppError::new(status, "cloud_grpc_error", error.message().to_string())
}
