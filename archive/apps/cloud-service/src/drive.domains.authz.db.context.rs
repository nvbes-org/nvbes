use sqlx::Row;
use uuid::Uuid;

use crate::{
    domains::authz::types::{ResourceContext, WorkspaceRole, parse_role},
    http::error::AppError,
};

use super::workspace_scope::owns_resource;

pub async fn resource_context_for_object(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    actor_user_id: Uuid,
    actor_principal_id: Uuid,
    object_id: Uuid,
) -> Result<ResourceContext, AppError> {
    let mut tx = begin_resource_transaction(
        db,
        workspace_id,
        Some(actor_user_id),
        Some(actor_principal_id),
    )
    .await?;
    let row = sqlx::query(
        r#"
        SELECT created_by, created_by_principal_id
        FROM storage_objects
        WHERE id = $1 AND workspace_id = $2
        "#,
    )
    .bind(object_id)
    .bind(workspace_id)
    .fetch_optional(&mut *tx)
    .await?;

    let row = row.ok_or_else(|| AppError::not_found("object_not_found", "Object not found."))?;
    let created_by: Uuid = row.get("created_by");
    let created_by_principal_id: Uuid = row.get("created_by_principal_id");

    let context = ResourceContext {
        owns_resource: owns_resource(
            created_by,
            created_by_principal_id,
            actor_user_id,
            actor_principal_id,
        ),
        ..ResourceContext::default()
    };
    tx.commit().await?;
    Ok(context)
}

pub async fn resource_context_for_share_link(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    actor_user_id: Uuid,
    actor_principal_id: Uuid,
    share_link_id: Uuid,
) -> Result<ResourceContext, AppError> {
    let mut tx = begin_resource_transaction(
        db,
        workspace_id,
        Some(actor_user_id),
        Some(actor_principal_id),
    )
    .await?;
    let row = sqlx::query(
        r#"
        SELECT so.created_by, so.created_by_principal_id, wp.member_can_create_share_links
        FROM share_links sl
        INNER JOIN storage_objects so ON so.id = sl.storage_object_id
        INNER JOIN workspace_policies wp ON wp.workspace_id = sl.workspace_id
        WHERE sl.id = $1 AND sl.workspace_id = $2
        "#,
    )
    .bind(share_link_id)
    .bind(workspace_id)
    .fetch_optional(&mut *tx)
    .await?;

    let row =
        row.ok_or_else(|| AppError::not_found("share_link_not_found", "Share link not found."))?;

    let created_by: Uuid = row.get("created_by");
    let created_by_principal_id: Uuid = row.get("created_by_principal_id");
    let member_can_create_share_links: bool = row.get("member_can_create_share_links");

    let context = ResourceContext {
        owns_resource: owns_resource(
            created_by,
            created_by_principal_id,
            actor_user_id,
            actor_principal_id,
        ),
        member_share_links_enabled: member_can_create_share_links,
        ..ResourceContext::default()
    };
    tx.commit().await?;
    Ok(context)
}

pub async fn target_role_for_member(
    db: &sqlx::PgPool,
    workspace_id: Uuid,
    member_id: Uuid,
) -> Result<WorkspaceRole, AppError> {
    let mut tx = begin_resource_transaction(db, workspace_id, None, None).await?;
    let row = sqlx::query(
        r#"
        SELECT role::text AS role
        FROM workspace_memberships
        WHERE workspace_id = $1 AND user_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(member_id)
    .fetch_optional(&mut *tx)
    .await?;

    let row = row.ok_or_else(|| AppError::not_found("member_not_found", "Member not found."))?;
    let role = parse_role(row.get::<String, _>("role").as_str())?;
    tx.commit().await?;
    Ok(role)
}

async fn begin_resource_transaction<'a>(
    db: &'a sqlx::PgPool,
    workspace_id: Uuid,
    user_id: Option<Uuid>,
    principal_id: Option<Uuid>,
) -> Result<sqlx::Transaction<'a, sqlx::Postgres>, sqlx::Error> {
    let mut tx = db.begin().await?;
    nvbes_tenancy::set_transaction_rls_context(
        &mut tx,
        nvbes_tenancy::RlsContext {
            principal_id,
            user_id,
            tenant_id: None,
            workspace_id: Some(workspace_id),
        },
    )
    .await?;
    Ok(tx)
}
