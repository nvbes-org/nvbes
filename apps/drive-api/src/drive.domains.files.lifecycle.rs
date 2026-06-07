use uuid::Uuid;

use crate::{
    domains::authz::WorkspaceAccess, domains::files::models::StorageObjectStatus,
    domains::quotas::StorageReleasedUsageInput, http::error::AppError,
};
use sqlx::PgPool;

use super::types::{DeleteObjectResponse, ObjectResponse, TrashListResponse};
use super::{db_lifecycle as db, queries};

pub async fn list_trash(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<TrashListResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let objects = db::list_trash_tx(&mut tx, access.workspace_id).await?;
    tx.commit().await?;

    Ok(TrashListResponse {
        objects: objects.into_iter().map(Into::into).collect(),
    })
}

pub async fn trash_object(
    db: &PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let object =
        queries::fetch_object_for_update_tx(&mut tx, access.workspace_id, object_id).await?;

    if matches!(object.status, StorageObjectStatus::Trashed) {
        return Err(AppError::conflict(
            "object_already_trashed",
            "Object is already in trash.",
        ));
    }

    object.ensure_mutable()?;

    let records = db::trash_subtree_tx(&mut tx, access.workspace_id, object_id).await?;
    let updated = records
        .into_iter()
        .find(|record| record.id == object_id)
        .ok_or_else(|| AppError::internal("trash_failed", "Failed to trash object."))?;

    queries::insert_audit_event_tx(
        &mut tx,
        queries::AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "file.trashed",
            target_type: "storage_object",
            target_id: Some(object_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "object_type": object.object_type.as_str(),
                "parent_id": object.parent_id,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(ObjectResponse {
        object: updated.into(),
    })
}

pub async fn restore_object(
    db: &PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ObjectResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let object =
        queries::fetch_object_for_update_tx(&mut tx, access.workspace_id, object_id).await?;

    if !matches!(object.status, StorageObjectStatus::Trashed) {
        return Err(AppError::conflict(
            "object_not_trashed",
            "Only trashed objects can be restored.",
        ));
    }

    if let Some(parent_id) = object.parent_id {
        queries::ensure_parent_is_active_folder_tx(&mut tx, access.workspace_id, parent_id).await?;
    }

    queries::ensure_name_available_tx(
        &mut tx,
        access.workspace_id,
        object.parent_id,
        &object.name,
        Some(object.id),
    )
    .await?;

    let records = db::restore_subtree_tx(&mut tx, access.workspace_id, object_id).await?;
    let updated = records
        .into_iter()
        .find(|record| record.id == object_id)
        .ok_or_else(|| AppError::internal("restore_failed", "Failed to restore object."))?;

    queries::insert_audit_event_tx(
        &mut tx,
        queries::AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "file.restored",
            target_type: "storage_object",
            target_id: Some(object_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "object_type": object.object_type.as_str(),
                "parent_id": object.parent_id,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(ObjectResponse {
        object: updated.into(),
    })
}

pub async fn delete_object(
    db: &PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    expected_etag: Option<String>,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<DeleteObjectResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let object =
        queries::fetch_object_for_update_tx(&mut tx, access.workspace_id, object_id).await?;
    super::etag::ensure_object_etag(expected_etag.as_deref(), &object)?;

    if !matches!(object.status, StorageObjectStatus::Trashed) {
        return Err(AppError::conflict(
            "object_not_trashed",
            "Object must be in trash before permanent deletion.",
        ));
    }

    let impact = db::compute_delete_impact_tx(&mut tx, access.workspace_id, object_id).await?;

    db::mark_subtree_deleted_tx(&mut tx, access.workspace_id, object_id).await?;

    crate::domains::quotas::release_storage_tx(
        &mut tx,
        StorageReleasedUsageInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            target_id: object_id,
            source: "file.delete_permanent",
            released_storage_bytes: impact.released_storage_bytes,
            deleted_file_count: impact.deleted_file_count,
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await?;

    queries::insert_audit_event_tx(
        &mut tx,
        queries::AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "file.deleted_permanently",
            target_type: "storage_object",
            target_id: Some(object_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "object_type": object.object_type.as_str(),
                "deleted_object_count": impact.deleted_object_count,
                "deleted_file_count": impact.deleted_file_count,
                "released_storage_bytes": impact.released_storage_bytes,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(DeleteObjectResponse {
        deleted_object_id: object_id,
        deleted_object_count: impact.deleted_object_count,
        deleted_file_count: impact.deleted_file_count,
        released_storage_bytes: impact.released_storage_bytes,
    })
}
