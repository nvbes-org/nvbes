use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub use super::models::*;
use super::types::*;
use crate::http::error::AppError;

pub async fn insert_share_link_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    storage_object_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
    max_downloads: Option<i32>,
    created_by: Uuid,
    created_by_principal_id: Uuid,
) -> Result<ShareLinkRecord, AppError> {
    let link = sqlx::query_as::<_, ShareLinkRecord>(
        r#"
        INSERT INTO share_links (
          workspace_id,
          storage_object_id,
          token_hash,
          permission,
          expires_at,
          max_downloads,
          created_by,
          created_by_principal_id
        )
        VALUES ($1, $2, $3, 'download', $4, $5, $6, $7)
        RETURNING
          id,
          workspace_id,
          storage_object_id,
          permission,
          expires_at,
          max_downloads,
          download_count,
          created_by,
          created_by_principal_id,
          revoked_at,
          created_at,
          updated_at
        "#,
    )
    .bind(workspace_id)
    .bind(storage_object_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(max_downloads)
    .bind(created_by)
    .bind(created_by_principal_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(link)
}

pub async fn update_share_link_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    share_link_id: Uuid,
    expires_at: DateTime<Utc>,
    max_downloads: Option<i32>,
) -> Result<ShareLinkRecord, AppError> {
    let link = sqlx::query_as::<_, ShareLinkRecord>(
        r#"
        UPDATE share_links
        SET expires_at = $3,
            max_downloads = $4,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND id = $2
        RETURNING
          id,
          workspace_id,
          storage_object_id,
          permission,
          expires_at,
          max_downloads,
          download_count,
          created_by,
          created_by_principal_id,
          revoked_at,
          created_at,
          updated_at
        "#,
    )
    .bind(workspace_id)
    .bind(share_link_id)
    .bind(expires_at)
    .bind(max_downloads)
    .fetch_one(&mut **tx)
    .await?;

    Ok(link)
}

pub async fn revoke_share_link_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    share_link_id: Uuid,
    revoked_at: DateTime<Utc>,
) -> Result<ShareLinkRecord, AppError> {
    let link = sqlx::query_as::<_, ShareLinkRecord>(
        r#"
        UPDATE share_links
        SET revoked_at = $3,
            updated_at = NOW()
        WHERE workspace_id = $1
          AND id = $2
        RETURNING
          id,
          workspace_id,
          storage_object_id,
          permission,
          expires_at,
          max_downloads,
          download_count,
          created_by,
          created_by_principal_id,
          revoked_at,
          created_at,
          updated_at
        "#,
    )
    .bind(workspace_id)
    .bind(share_link_id)
    .bind(revoked_at)
    .fetch_one(&mut **tx)
    .await?;

    Ok(link)
}

pub async fn increment_download_count_tx(
    tx: &mut Transaction<'_, Postgres>,
    share_link_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE share_links
        SET download_count = download_count + 1,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(share_link_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub fn share_link_permission_as_str(permission: ShareLinkPermission) -> &'static str {
    match permission {
        ShareLinkPermission::Download => "download",
    }
}

pub fn map_record_to_view(record: ShareLinkRecord) -> ShareLinkView {
    ShareLinkView {
        id: record.id,
        workspace_id: record.workspace_id,
        storage_object_id: record.storage_object_id,
        permission: share_link_permission_as_str(record.permission).to_owned(),
        expires_at: record.expires_at,
        max_downloads: record.max_downloads,
        download_count: record.download_count,
        created_by: record.created_by,
        created_by_principal_id: record.created_by_principal_id,
        revoked_at: record.revoked_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

pub fn map_public_record_to_view(record: PublicShareRecord) -> PublicShareView {
    PublicShareView {
        share_link_id: record.share_link_id,
        object_id: record.storage_object_id,
        name: record.name,
        mime_type: record.mime_type,
        size_bytes: record.size_bytes,
        expires_at: record.expires_at,
        remaining_downloads: record
            .max_downloads
            .map(|limit| (limit - record.download_count).max(0)),
    }
}
