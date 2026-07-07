use tonic::Status;

use crate::grpc::{
    pb::nvbes::{cloud::v1 as cloud, platform::v1},
    service_status::{non_empty, parse_uuid, sql_status, validate_context},
    service_workspace_rows::WorkspaceInvitationRow,
};

pub async fn list_invitations(
    db: &sqlx::PgPool,
    request: cloud::ListWorkspaceInvitationsRequest,
) -> Result<cloud::ListWorkspaceInvitationsResponse, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    validate_context(request.context.as_ref(), Some(workspace_id))?;
    let statuses = request
        .statuses
        .into_iter()
        .map(|status| status.trim().to_lowercase())
        .filter(|status| !status.is_empty())
        .collect::<Vec<_>>();
    let invitations = sqlx::query_as::<_, WorkspaceInvitationRow>(
        r#"
        SELECT id AS invitation_id, workspace_id, email, role::text AS role, status::text AS status, expires_at, accepted_at, revoked_at, created_at
        FROM workspace_invitations
        WHERE workspace_id = $1
          AND (cardinality($2::text[]) = 0 OR status::text = ANY($2))
        ORDER BY created_at DESC
        "#,
    )
    .bind(workspace_id)
    .bind(statuses)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(WorkspaceInvitationRow::into_proto)
    .collect();

    Ok(cloud::ListWorkspaceInvitationsResponse {
        invitations,
        page: Some(v1::CursorPageResponse {
            next_cursor: String::new(),
            has_more: false,
        }),
    })
}

pub async fn get_invitation_by_token_hash(
    db: &sqlx::PgPool,
    request: cloud::GetWorkspaceInvitationByTokenHashRequest,
) -> Result<cloud::WorkspaceInvitation, Status> {
    validate_context(request.context.as_ref(), None)?;
    let token_hash = non_empty(request.token_hash, "token_hash")?;
    let invitation = sqlx::query_as::<_, WorkspaceInvitationRow>(
        r#"
        SELECT id AS invitation_id, workspace_id, email, role::text AS role, status::text AS status, expires_at, accepted_at, revoked_at, created_at
        FROM workspace_invitations
        WHERE token_hash = $1
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("workspace invitation not found"))?;

    Ok(invitation.into_proto())
}
