use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::models::{PublicShareRecord, ShareLinkRecord, SharePolicy, ShareableObjectRecord};
use crate::http::error::AppError;

pub async fn ensure_shareable_object_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    object_id: Uuid,
) -> Result<ShareableObjectRecord, AppError> {
    let object = sqlx::query_as::<_, ShareableObjectRecord>(
        r#"
        SELECT
          id,
          status,
          scan_status
        FROM storage_objects
        WHERE workspace_id = $1
          AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(workspace_id)
    .bind(object_id)
    .fetch_optional(&mut **tx)
    .await?;

    object.ok_or_else(|| AppError::not_found("object_not_found", "Object not found."))
}

pub async fn fetch_share_policy_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
) -> Result<SharePolicy, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          wp.default_share_link_ttl_days,
          wp.max_share_link_ttl_days,
          p.max_share_links
        FROM workspace_policies wp
        INNER JOIN workspaces w ON w.id = wp.workspace_id
        INNER JOIN plans p ON p.id = w.plan_id
        WHERE wp.workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(&mut **tx)
    .await?;

    let row =
        row.ok_or_else(|| AppError::not_found("workspace_not_found", "Workspace not found."))?;

    Ok(SharePolicy {
        default_ttl_days: row.get("default_share_link_ttl_days"),
        max_ttl_days: row.get("max_share_link_ttl_days"),
        max_share_links: row.get("max_share_links"),
    })
}

pub async fn enforce_share_link_capacity_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    max_share_links: i32,
) -> Result<(), AppError> {
    let active_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM share_links
        WHERE workspace_id = $1
          AND revoked_at IS NULL
          AND expires_at > NOW()
        "#,
    )
    .bind(workspace_id)
    .fetch_one(&mut **tx)
    .await?;

    if active_count >= i64::from(max_share_links) {
        return Err(AppError::conflict(
            "share_link_limit_reached",
            "Workspace has reached its active share link limit.",
        ));
    }

    Ok(())
}

pub async fn fetch_share_link_for_update_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    share_link_id: Uuid,
) -> Result<ShareLinkRecord, AppError> {
    let link = sqlx::query_as::<_, ShareLinkRecord>(
        r#"
        SELECT
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
        FROM share_links
        WHERE workspace_id = $1
          AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(workspace_id)
    .bind(share_link_id)
    .fetch_optional(&mut **tx)
    .await?;

    link.ok_or_else(|| AppError::not_found("share_link_not_found", "Share link not found."))
}

pub async fn resolve_public_share_for_update_tx(
    tx: &mut Transaction<'_, Postgres>,
    token_hash: &str,
) -> Result<PublicShareRecord, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          sl.id AS share_link_id,
          sl.workspace_id,
          sl.storage_object_id,
          sl.expires_at,
          sl.max_downloads,
          sl.download_count,
          sl.revoked_at,
          so.name,
          so.mime_type,
          so.size_bytes,
          so.status AS object_status,
          so.scan_status,
          so.object_key
        FROM share_links sl
        INNER JOIN storage_objects so ON so.id = sl.storage_object_id
        WHERE sl.token_hash = $1
        FOR UPDATE OF sl
        "#,
    )
    .bind(token_hash)
    .fetch_optional(&mut **tx)
    .await?;

    let row =
        row.ok_or_else(|| AppError::not_found("share_not_found", "Public share not found."))?;

    use sqlx::Row;
    Ok(PublicShareRecord {
        share_link_id: row.get("share_link_id"),
        workspace_id: row.get("workspace_id"),
        storage_object_id: row.get("storage_object_id"),
        expires_at: row.get("expires_at"),
        max_downloads: row.get("max_downloads"),
        download_count: row.get("download_count"),
        revoked_at: row.get("revoked_at"),
        name: row.get("name"),
        mime_type: row.get("mime_type"),
        size_bytes: row.get("size_bytes"),
        object_status: row.get("object_status"),
        scan_status: row.get("scan_status"),
        object_key: row.get("object_key"),
    })
}

pub async fn list_share_links(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> Result<Vec<ShareLinkRecord>, AppError> {
    let links = sqlx::query_as::<_, ShareLinkRecord>(
        r#"
        SELECT
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
        FROM share_links
        WHERE workspace_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(links)
}
