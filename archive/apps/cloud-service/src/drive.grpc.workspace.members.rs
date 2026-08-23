use tonic::Status;

use crate::grpc::{
    pb::nvbes::{cloud::v1 as cloud, platform::v1},
    service_status::{non_empty, optional_string, parse_uuid, sql_status},
    service_workspace_context::begin_request_transaction,
    service_workspace_persistence::{fetch_member, update_membership, upsert_membership},
    service_workspace_principals::ensure_principal,
    service_workspace_rows::WorkspaceMemberRow,
};

pub async fn add_member(
    db: &sqlx::PgPool,
    request: cloud::AddWorkspaceMemberRequest,
) -> Result<cloud::WorkspaceMember, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    let role = non_empty(request.role, "role")?;
    let source = optional_string(request.source).unwrap_or_else(|| "manual".to_string());
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
        &source,
    )
    .await?;
    let member = fetch_member(&mut tx, workspace_id, principal_id).await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(member)
}

pub async fn update_member(
    db: &sqlx::PgPool,
    request: cloud::UpdateWorkspaceMemberRequest,
) -> Result<cloud::WorkspaceMember, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    let role = optional_string(request.role);
    let status = optional_string(request.status);
    let source = optional_string(request.source);
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
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
    let member = fetch_member(&mut tx, workspace_id, principal_id).await?;
    tx.commit().await.map_err(sql_status)?;
    Ok(member)
}

pub async fn remove_member(
    db: &sqlx::PgPool,
    request: cloud::RemoveWorkspaceMemberRequest,
) -> Result<cloud::WorkspaceMembershipRemoval, Status> {
    let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
    let principal_id = parse_uuid(&request.principal_id, "principal_id")?;
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
    let result = sqlx::query(
        "UPDATE workspace_memberships
         SET status = 'removed', updated_at = NOW()
         WHERE workspace_id = $1 AND user_id = $2",
    )
    .bind(workspace_id)
    .bind(principal_id)
    .execute(&mut *tx)
    .await
    .map_err(sql_status)?;
    tx.commit().await.map_err(sql_status)?;
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
    let (mut tx, _) =
        begin_request_transaction(db, request.context.as_ref(), Some(workspace_id)).await?;
    let members = sqlx::query_as::<_, WorkspaceMemberRow>(
        r#"
        SELECT workspace_id, user_id AS principal_id, role::text AS role,
               status::text AS status, source::text AS source, created_at AS joined_at, updated_at
        FROM workspace_memberships
        WHERE workspace_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(WorkspaceMemberRow::into_proto)
    .collect();
    tx.commit().await.map_err(sql_status)?;
    Ok(cloud::ListWorkspaceMembersResponse {
        members,
        page: Some(v1::CursorPageResponse {
            next_cursor: String::new(),
            has_more: false,
        }),
    })
}
