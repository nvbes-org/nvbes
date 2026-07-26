use tonic::Status;

use nvbes_core::pagination::{KeysetCursor, decode_cursor, encode_cursor, page_from_rows};

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
    let limit = request
        .page
        .as_ref()
        .map(|page| page.limit as i64)
        .unwrap_or(50)
        .clamp(1, 200);
    let cursor = request
        .page
        .as_ref()
        .and_then(|page| non_empty(page.cursor.clone(), "cursor").ok())
        .map(|value| decode_cursor::<KeysetCursor>(&value))
        .transpose()
        .map_err(|_| Status::invalid_argument("invalid pagination cursor"))?;
    let invitations = sqlx::query_as::<_, WorkspaceInvitationRow>(
        r#"
        SELECT id AS invitation_id, workspace_id, email, role::text AS role, status::text AS status, expires_at, accepted_at, revoked_at, created_at
        FROM workspace_invitations
        WHERE workspace_id = $1
          AND (
            $2::timestamp with time zone IS NULL
            OR (created_at, id) < ($2, $3)
          )
          AND (cardinality($4::text[]) = 0 OR status::text = ANY($4))
        ORDER BY created_at DESC, id DESC
        LIMIT $5
        "#,
    )
    .bind(workspace_id)
    .bind(cursor.as_ref().map(|value| value.created_at))
    .bind(cursor.as_ref().map(|value| value.id))
    .bind(statuses)
    .bind(limit + 1)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    ;
    let page = page_from_rows(invitations, limit as usize, |row| KeysetCursor {
        created_at: row.created_at,
        id: row.invitation_id,
    });

    Ok(cloud::ListWorkspaceInvitationsResponse {
        invitations: page
            .items
            .into_iter()
            .map(WorkspaceInvitationRow::into_proto)
            .collect(),
        page: Some(v1::CursorPageResponse {
            next_cursor: page
                .next_cursor
                .as_ref()
                .map(encode_cursor)
                .transpose()
                .map_err(|_| Status::internal("pagination cursor could not be encoded"))?
                .unwrap_or_default(),
            has_more: page.has_more,
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
