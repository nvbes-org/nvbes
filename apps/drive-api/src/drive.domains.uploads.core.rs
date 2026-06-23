use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domains::authz::WorkspaceAccess, http::error::AppError};

use super::db;
use super::logic;
use super::queries;
use super::types::*;

pub async fn create_upload(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &PgPool,
    access: &WorkspaceAccess,
    input: CreateUploadInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<CreateUploadResponse, AppError> {
    let name = logic::validate_object_name(&input.name)?;
    let mime_type = logic::validate_mime_type(&input.mime_type)?;
    let expected_size_bytes = logic::validate_size(input.expected_size_bytes)?;
    let expected_checksum = logic::normalize_checksum(input.expected_checksum)?;

    if let Some(parent_id) = input.parent_id {
        queries::ensure_parent_is_active_folder(db, access.workspace_id, parent_id).await?;
    }

    let now = Utc::now();
    let expires_at = now + Duration::minutes(15);
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    crate::domains::quotas::ensure_upload_allowed_tx(
        &mut tx,
        access.workspace_id,
        expected_size_bytes,
    )
    .await?;

    queries::ensure_name_available_tx(&mut tx, access.workspace_id, input.parent_id, &name, None)
        .await?;

    let storage_object_id = Uuid::new_v4();
    let upload_id = Uuid::new_v4();
    let object_key = logic::build_object_key(access.workspace_id, storage_object_id, upload_id);

    let storage_object = db::insert_pending_storage_object_tx(
        &mut tx,
        storage_object_id,
        access.workspace_id,
        input.parent_id,
        &name,
        &mime_type,
        &object_key,
        access.auth.user_id,
        access.auth.principal_id,
    )
    .await?;

    db::insert_upload_session_tx(
        &mut tx,
        upload_id,
        access.workspace_id,
        storage_object_id,
        access.auth.user_id,
        access.auth.principal_id,
        expected_size_bytes,
        expected_checksum.as_deref(),
        expires_at,
    )
    .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "upload.created",
            target_type: "upload_session",
            target_id: Some(upload_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "storage_object_id": storage_object_id,
                "parent_id": input.parent_id,
                "name": name,
                "expected_size_bytes": expected_size_bytes,
                "expected_checksum": expected_checksum,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    let upload_url = logic::build_signed_upload_url(
        storage,
        &object_key,
        &mime_type,
        expected_size_bytes,
        expires_at,
    )
    .await?;

    Ok(CreateUploadResponse {
        upload_id,
        storage_object: db::map_record_to_view(storage_object),
        upload_url,
        tus_url: logic::build_tus_upload_url(access.workspace_id, upload_id),
        expires_at,
    })
}

pub async fn create_tus_upload(
    storage: &dyn nvbes_storage::ObjectStore,
    db: &PgPool,
    access: &WorkspaceAccess,
    input: CreateTusUploadInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<CreateUploadResponse, AppError> {
    let name = logic::validate_object_name(&input.name)?;
    let mime_type = logic::validate_mime_type(&input.mime_type)?;
    let expected_size_bytes = logic::validate_size(input.upload_length)?;
    let expected_checksum = logic::normalize_checksum(input.expected_checksum)?;

    if let Some(parent_id) = input.parent_id {
        queries::ensure_parent_is_active_folder(db, access.workspace_id, parent_id).await?;
    }

    let now = Utc::now();
    let expires_at = now + Duration::minutes(60);
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;

    crate::domains::quotas::ensure_upload_allowed_tx(
        &mut tx,
        access.workspace_id,
        expected_size_bytes,
    )
    .await?;

    queries::ensure_name_available_tx(&mut tx, access.workspace_id, input.parent_id, &name, None)
        .await?;

    let storage_object_id = Uuid::new_v4();
    let upload_id = Uuid::new_v4();
    let object_key = logic::build_object_key(access.workspace_id, storage_object_id, upload_id);

    let multipart_upload_id = storage
        .create_multipart_upload(&object_key, Some(&mime_type))
        .await
        .map_err(|e| {
            AppError::internal(
                "storage_multipart_create_failed",
                format!("Failed to create multipart upload: {e}"),
            )
        })?;

    let storage_object = db::insert_pending_storage_object_tx(
        &mut tx,
        storage_object_id,
        access.workspace_id,
        input.parent_id,
        &name,
        &mime_type,
        &object_key,
        access.auth.user_id,
        access.auth.principal_id,
    )
    .await?;

    db::insert_upload_session_tx(
        &mut tx,
        upload_id,
        access.workspace_id,
        storage_object_id,
        access.auth.user_id,
        access.auth.principal_id,
        expected_size_bytes,
        expected_checksum.as_deref(),
        expires_at,
    )
    .await?;

    db::attach_multipart_upload_tx(
        &mut tx,
        access.workspace_id,
        upload_id,
        &multipart_upload_id,
    )
    .await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "upload.tus.created",
            target_type: "upload_session",
            target_id: Some(upload_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "storage_object_id": storage_object_id,
                "parent_id": input.parent_id,
                "name": name,
                "expected_size_bytes": expected_size_bytes,
                "expected_checksum": expected_checksum,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    let tus_url = logic::build_tus_upload_url(access.workspace_id, upload_id);

    Ok(CreateUploadResponse {
        upload_id,
        storage_object: db::map_record_to_view(storage_object),
        upload_url: SignedUploadUrlView {
            url: tus_url.clone(),
            method: "PATCH",
            expires_at,
            required_headers: vec![
                RequiredHeaderView {
                    name: "Tus-Resumable".to_owned(),
                    value: "1.0.0".to_owned(),
                },
                RequiredHeaderView {
                    name: "Content-Type".to_owned(),
                    value: "application/offset+octet-stream".to_owned(),
                },
                RequiredHeaderView {
                    name: "Content-Encoding".to_owned(),
                    value: logic::required_content_encoding().to_owned(),
                },
            ],
        },
        tus_url,
        expires_at,
    })
}
