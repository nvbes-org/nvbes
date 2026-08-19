#[path = "drive.grpc.workspace.tests.support.rs"]
mod support;

use anyhow::ensure;
use tonic::Code;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::{
        cloud::v1 as cloud,
        platform::v1::{RequestContext, TenantContext},
    },
    service_workspace,
};
use support::GrpcTestDatabase;

#[tokio::test]
async fn grpc_rls_workspace_operations_use_one_scoped_transaction() {
    let Ok(database) = GrpcTestDatabase::create().await else {
        eprintln!("skipping test: isolated Cloud gRPC database is not available");
        return;
    };
    let result = exercise_workspace_operations(&database.runtime).await;
    let cleanup = database.cleanup().await;

    result.expect("all Cloud workspace RPC operations must satisfy forced RLS");
    cleanup.expect("isolated Cloud gRPC database should be removed");
}

async fn exercise_workspace_operations(db: &sqlx::PgPool) -> anyhow::Result<()> {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let member_id = Uuid::new_v4();
    let token_hash = format!("grpc-invitation-{}", Uuid::new_v4());

    let active_role: String = sqlx::query_scalar("SELECT current_user::text")
        .fetch_one(db)
        .await?;
    ensure!(active_role == "nvbes_app", "test must exercise nvbes_app");

    let created = service_workspace::create_workspace(
        db,
        cloud::CreateWorkspaceRequest {
            context: Some(request_context(
                Some(tenant_id),
                Some(workspace_id),
                owner_id,
            )),
            name: "RLS workspace".to_string(),
            region_id: "eu".to_string(),
            data_residency: "eu".to_string(),
            workspace_id: workspace_id.to_string(),
            tenant_id: tenant_id.to_string(),
            owner_principal_id: owner_id.to_string(),
            workspace_type: "team".to_string(),
            plan_code: "team_plus".to_string(),
            owner_role: "owner".to_string(),
            membership_source: "system".to_string(),
            owner_principal: Some(principal(owner_id, "owner@example.test")),
            ..Default::default()
        },
    )
    .await?;
    ensure!(created.workspace_id == workspace_id.to_string());

    let visible_without_context: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workspaces WHERE id = $1")
            .bind(workspace_id)
            .fetch_one(db)
            .await?;
    ensure!(
        visible_without_context == 0,
        "transaction-local workspace context must not leak back to the pool"
    );

    let fetched = service_workspace::get_workspace(
        db,
        cloud::GetWorkspaceRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            workspace_id: workspace_id.to_string(),
        },
    )
    .await?;
    ensure!(fetched.tenant_id == tenant_id.to_string());

    let wrong_tenant = service_workspace::get_workspace(
        db,
        cloud::GetWorkspaceRequest {
            context: Some(request_context(
                Some(Uuid::new_v4()),
                Some(workspace_id),
                owner_id,
            )),
            workspace_id: workspace_id.to_string(),
        },
    )
    .await
    .expect_err("cross-tenant context must be rejected");
    ensure!(wrong_tenant.code() == Code::PermissionDenied);

    let updated = service_workspace::update_workspace(
        db,
        cloud::UpdateWorkspaceRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            workspace_id: workspace_id.to_string(),
            name: "Scoped workspace".to_string(),
            ..Default::default()
        },
    )
    .await?;
    ensure!(updated.name == "Scoped workspace");

    let workspaces = service_workspace::list_workspaces(
        db,
        cloud::ListWorkspacesRequest {
            context: Some(request_context(Some(tenant_id), None, owner_id)),
            tenant_id: tenant_id.to_string(),
            ..Default::default()
        },
    )
    .await?;
    ensure!(
        workspaces
            .workspaces
            .iter()
            .any(|workspace| workspace.workspace_id == workspace_id.to_string())
    );

    service_workspace::add_member(
        db,
        cloud::AddWorkspaceMemberRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            workspace_id: workspace_id.to_string(),
            principal_id: member_id.to_string(),
            role: "member".to_string(),
            source: "system".to_string(),
            principal: Some(principal(member_id, "member@example.test")),
        },
    )
    .await?;
    let updated_member = service_workspace::update_member(
        db,
        cloud::UpdateWorkspaceMemberRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            workspace_id: workspace_id.to_string(),
            principal_id: member_id.to_string(),
            role: "admin".to_string(),
            ..Default::default()
        },
    )
    .await?;
    ensure!(updated_member.role == "admin");
    let members = service_workspace::list_members(
        db,
        cloud::ListWorkspaceMembersRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            workspace_id: workspace_id.to_string(),
            page: None,
        },
    )
    .await?;
    ensure!(members.members.len() == 2);

    let invitation = service_workspace::create_invitation(
        db,
        cloud::CreateWorkspaceInvitationRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            workspace_id: workspace_id.to_string(),
            email: "invitee@example.test".to_string(),
            role: "member".to_string(),
            invited_by_principal_id: owner_id.to_string(),
            token_hash: token_hash.clone(),
            invited_by_principal: Some(principal(owner_id, "owner@example.test")),
            ..Default::default()
        },
    )
    .await?;
    let invitations = service_workspace::list_invitations(
        db,
        cloud::ListWorkspaceInvitationsRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            workspace_id: workspace_id.to_string(),
            ..Default::default()
        },
    )
    .await?;
    ensure!(invitations.invitations.len() == 1);

    let missing_token_scope = service_workspace::get_invitation_by_token_hash(
        db,
        cloud::GetWorkspaceInvitationByTokenHashRequest {
            context: Some(request_context(None, None, owner_id)),
            token_hash: token_hash.clone(),
        },
    )
    .await
    .expect_err("token lookup must require an explicit workspace scope");
    ensure!(missing_token_scope.code() == Code::InvalidArgument);
    let fetched_invitation = service_workspace::get_invitation_by_token_hash(
        db,
        cloud::GetWorkspaceInvitationByTokenHashRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            token_hash,
        },
    )
    .await?;
    ensure!(fetched_invitation.invitation_id == invitation.invitation_id);

    service_workspace::accept_invitation(
        db,
        cloud::AcceptWorkspaceInvitationRequest {
            context: Some(request_context(None, Some(workspace_id), member_id)),
            invitation_id: invitation.invitation_id,
            workspace_id: workspace_id.to_string(),
            principal_id: member_id.to_string(),
            role: "member".to_string(),
            principal: Some(principal(member_id, "member@example.test")),
            ..Default::default()
        },
    )
    .await?;
    let removal = service_workspace::remove_member(
        db,
        cloud::RemoveWorkspaceMemberRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            workspace_id: workspace_id.to_string(),
            principal_id: member_id.to_string(),
            reason: "test".to_string(),
        },
    )
    .await?;
    ensure!(removal.removed);

    service_workspace::delete_workspace(
        db,
        cloud::DeleteWorkspaceRequest {
            context: Some(request_context(None, Some(workspace_id), owner_id)),
            workspace_id: workspace_id.to_string(),
            reason: "test".to_string(),
        },
    )
    .await?;

    let nil = Uuid::nil().to_string();
    let settings: (String, String, String, String) = sqlx::query_as(
        "SELECT current_setting('nvbes.principal_id', true),
                current_setting('nvbes.user_id', true),
                current_setting('nvbes.tenant_id', true),
                current_setting('nvbes.workspace_id', true)",
    )
    .fetch_one(db)
    .await?;
    ensure!(
        settings == (nil.clone(), nil.clone(), nil.clone(), nil),
        "committed RPC transactions must restore the pool's fail-closed sentinels"
    );
    Ok(())
}

fn request_context(
    tenant_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
    actor_principal_id: Uuid,
) -> RequestContext {
    RequestContext {
        request_id: Uuid::new_v4().to_string(),
        correlation_id: Uuid::new_v4().to_string(),
        actor_principal_id: actor_principal_id.to_string(),
        tenant: Some(TenantContext {
            tenant_id: tenant_id.map(|id| id.to_string()).unwrap_or_default(),
            workspace_id: workspace_id.map(|id| id.to_string()).unwrap_or_default(),
            region_id: "eu".to_string(),
            data_residency: "eu".to_string(),
        }),
    }
}

fn principal(principal_id: Uuid, email: &str) -> cloud::PrincipalProjection {
    cloud::PrincipalProjection {
        principal_id: principal_id.to_string(),
        email: email.to_string(),
        display_name: email.to_string(),
        principal_kind: "human".to_string(),
    }
}
