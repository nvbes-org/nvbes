use super::models::StorageObjectRecord;
use super::pagination::{ObjectListCursor, normalize_name_prefix};
use super::types::*;
use super::{db, queries};
use crate::{
    domains::authz::WorkspaceAccess, domains::files::models::StorageObjectType,
    http::error::AppError,
};
use nvbes_core::pagination::page_from_rows;
use sqlx::PgPool;
use uuid::Uuid;

#[cfg(test)]
#[path = "drive.domains.files.core.db_tests.rs"]
mod db_tests;

pub async fn list_objects(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: ListObjectsInput,
) -> Result<ListObjectsResponse, AppError> {
    let ListObjectsInput {
        parent_id,
        limit,
        cursor,
        object_type,
        name_prefix,
    } = input;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let cursor = cursor
        .as_deref()
        .map(ObjectListCursor::decode)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let name_prefix = normalize_name_prefix(name_prefix);
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    if let Some(parent_id) = parent_id {
        queries::ensure_parent_is_active_folder_tx(&mut tx, access.workspace_id, parent_id).await?;
    }

    let objects = queries::list_objects_tx(
        &mut tx,
        access.workspace_id,
        parent_id,
        cursor.as_ref(),
        name_prefix.as_deref(),
        object_type.map(ObjectTypeFilter::as_str),
        limit + 1,
    )
    .await?;
    tx.commit().await?;
    let page = page_from_rows(objects, limit as usize, ObjectListCursor::from_record);

    Ok(ListObjectsResponse {
        parent_id,
        objects: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page
            .next_cursor
            .as_ref()
            .map(ObjectListCursor::encode)
            .transpose()
            .map_err(|_| {
                AppError::internal(
                    "cursor_encoding_failed",
                    "Pagination cursor could not be encoded.",
                )
            })?,
        has_more: page.has_more,
    })
}

pub async fn create_folder(
    db: &PgPool,
    access: &WorkspaceAccess,
    input: CreateFolderInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    let name = StorageObjectRecord::validate_name(&input.name)?;

    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    if let Some(parent_id) = input.parent_id {
        queries::ensure_parent_is_active_folder_tx(&mut tx, access.workspace_id, parent_id).await?;
    }

    queries::ensure_name_available_tx(&mut tx, access.workspace_id, input.parent_id, &name, None)
        .await?;

    let folder = db::insert_folder_tx(
        &mut tx,
        access.workspace_id,
        input.parent_id,
        &name,
        access.auth.user_id,
        access.auth.principal_id,
    )
    .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "folder.created",
            target_type: "storage_object",
            target_id: Some(folder.id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "object_type": "folder",
                "parent_id": input.parent_id,
                "name": name,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(ObjectResponse {
        object: folder.into(),
    })
}

pub async fn rename_object(
    db: &PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    input: RenameObjectInput,
    expected_etag: Option<String>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    let name = StorageObjectRecord::validate_name(&input.name)?;
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    let object =
        queries::fetch_object_for_update_tx(&mut tx, access.workspace_id, object_id).await?;
    super::etag::ensure_object_etag(expected_etag.as_deref(), &object)?;
    object.ensure_mutable()?;

    if object.name != name {
        queries::ensure_name_available_tx(
            &mut tx,
            access.workspace_id,
            object.parent_id,
            &name,
            Some(object.id),
        )
        .await?;
    }

    let updated = db::update_object_name_tx(&mut tx, access.workspace_id, object_id, &name).await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "file.renamed",
            target_type: "storage_object",
            target_id: Some(object_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "previous_name": object.name,
                "name": name,
                "object_type": object.object_type.as_str(),
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(ObjectResponse {
        object: updated.into(),
    })
}

pub async fn move_object(
    db: &PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    input: MoveObjectInput,
    expected_etag: Option<String>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    let object =
        queries::fetch_object_for_update_tx(&mut tx, access.workspace_id, object_id).await?;
    super::etag::ensure_object_etag(expected_etag.as_deref(), &object)?;
    object.ensure_mutable()?;

    if input.destination_parent_id == Some(object.id) {
        return Err(AppError::bad_request(
            "invalid_parent",
            "An object cannot be moved into itself.",
        ));
    }

    if let Some(destination_parent_id) = input.destination_parent_id {
        queries::ensure_parent_is_active_folder_tx(
            &mut tx,
            access.workspace_id,
            destination_parent_id,
        )
        .await?;

        if matches!(object.object_type, StorageObjectType::Folder)
            && queries::subtree_contains_tx(
                &mut tx,
                access.workspace_id,
                object.id,
                destination_parent_id,
            )
            .await?
        {
            return Err(AppError::bad_request(
                "invalid_move",
                "A folder cannot be moved inside one of its descendants.",
            ));
        }
    }

    queries::ensure_name_available_tx(
        &mut tx,
        access.workspace_id,
        input.destination_parent_id,
        &object.name,
        Some(object.id),
    )
    .await?;

    let updated = db::update_object_parent_tx(
        &mut tx,
        access.workspace_id,
        object_id,
        input.destination_parent_id,
    )
    .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "file.moved",
            target_type: "storage_object",
            target_id: Some(object_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "from_parent_id": object.parent_id,
                "to_parent_id": input.destination_parent_id,
                "object_type": object.object_type.as_str(),
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(ObjectResponse {
        object: updated.into(),
    })
}
