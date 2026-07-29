use chrono::{Duration, Utc};
use tonic::Status;

use nvbes_core::pagination::{KeysetCursor, decode_cursor, encode_cursor, page_from_rows};

use crate::grpc::{
    pb::nvbes::{cloud::v1 as cloud, platform::v1},
    service_status::{non_empty, optional_datetime, parse_uuid, sql_status},
    service_workspace_context::{begin_request_transaction, required_context_workspace_id},
    service_workspace_persistence::upsert_membership,
    service_workspace_principals::ensure_principal,
    service_workspace_rows::WorkspaceInvitationRow,
};

pub async fn create_invitation(
    db: &sqlx::PgPool,
    request: cloud::CreateWorkspaceInvitationRequest,
) -> Result<cloud::WorkspaceInvitation, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    let invited_by = parse_uuid(&request.invited_by_principal_id, "invited_by_principal_id")?;
    let email = non_empty(request.email, "email")?.to_lowercase();
    let role = non_empty(request.role, "role")?;
    let token_hash = non_empty(request.token_hash, "token_hash")?;
    let expires_at = optional_datetime(&request.expires_at, "expires_at")?
        .unwrap_or_else(|| Utc::now() + Duration::days(7));
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
    ensure_principal(
        &mut tx,
        invited_by,
        request.invited_by_principal.as_ref(),
        "invited_by_principal_id",
    )
    .await?;
    let invitation = sqlx::query_as::<_, WorkspaceInvitationRow>(
        r#"
        INSERT INTO workspace_invitations (
          workspace_id, email, role, invited_by, token_hash, expires_at
        )
        VALUES ($1, $2, $3::workspace_member_role, $4, $5, $6)
        RETURNING id AS invitation_id, workspace_id, email, role::text AS role,
                  status::text AS status, expires_at, accepted_at, revoked_at, created_at
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

pub async fn list_invitations(
    db: &sqlx::PgPool,
    request: cloud::ListWorkspaceInvitationsRequest,
) -> Result<cloud::ListWorkspaceInvitationsResponse, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
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
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
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
    .fetch_all(&mut *tx)
    .await
    .map_err(sql_status)?;
    let page = page_from_rows(invitations, limit as usize, |row| KeysetCursor {
        created_at: row.created_at,
        id: row.invitation_id,
    });
    tx.commit().await.map_err(sql_status)?;

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
    let token_hash = non_empty(request.token_hash, "token_hash")?;
    let workspace_id = required_context_workspace_id(request.context.as_ref())?;
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
    let invitation = sqlx::query_as::<_, WorkspaceInvitationRow>(
        r#"
        SELECT id AS invitation_id, workspace_id, email, role::text AS role, status::text AS status, expires_at, accepted_at, revoked_at, created_at
        FROM workspace_invitations
        WHERE token_hash = $1
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(&mut *tx)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("workspace invitation not found"))?;
    tx.commit().await.map_err(sql_status)?;

    Ok(invitation.into_proto())
}

pub async fn accept_invitation(
    db: &sqlx::PgPool,
    request: cloud::AcceptWorkspaceInvitationRequest,
) -> Result<cloud::WorkspaceInvitationAcceptance, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    let invitation_id = parse_uuid(&request.invitation_id, "invitation_id")?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    let role = non_empty(request.role, "role")?;
    let accepted_at =
        optional_datetime(&request.accepted_at, "accepted_at")?.unwrap_or_else(Utc::now);
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
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
    let result = sqlx::query(
        "UPDATE workspace_invitations
         SET status = 'accepted', accepted_at = $2, updated_at = $2
         WHERE id = $1 AND workspace_id = $3",
    )
    .bind(invitation_id)
    .bind(accepted_at)
    .bind(workspace_id)
    .execute(&mut *tx)
    .await
    .map_err(sql_status)?;
    if result.rows_affected() == 0 {
        return Err(Status::not_found("workspace invitation not found"));
    }
    tx.commit().await.map_err(sql_status)?;
    Ok(cloud::WorkspaceInvitationAcceptance {
        invitation_id: invitation_id.to_string(),
        workspace_id: workspace_id.to_string(),
        principal_id: principal_id.to_string(),
        accepted: true,
    })
}
