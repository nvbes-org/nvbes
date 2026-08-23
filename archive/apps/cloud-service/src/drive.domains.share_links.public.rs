use chrono::{Duration as ChronoDuration, Utc};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::{domains::quotas::BandwidthOutUsageInput, http::error::AppError};

use super::db;
use super::logic;
use super::observability::{self, AuditEventInput};
use super::queries;
use super::types::*;

pub async fn get_public_share(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    token: &str,
    client_key: &str,
    require_clean_scan: bool,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<PublicShareResponse, AppError> {
    nvbes_core::limiter::check_rate_limit(
        redis,
        "public_share_resolve",
        client_key,
        30,
        Duration::from_secs(60),
    )
    .await?;

    let mut tx = db.begin().await?;
    let hash = nvbes_core::auth::token_hash(token.trim());
    set_public_share_rls_context(&mut tx, &hash).await?;
    let resolved = queries::resolve_public_share_for_update_tx(&mut tx, &hash).await?;
    if let Err(error) = logic::enforce_public_share_access(&resolved, require_clean_scan) {
        log_public_share_denied(
            &mut tx,
            &resolved,
            &error.code,
            ip.as_deref(),
            user_agent.as_deref(),
        )
        .await?;
        tx.commit().await?;
        return Err(error);
    }

    observability::log_share_link_event_tx(
        &mut tx,
        AuditEventInput {
            workspace_id: resolved.workspace_id,
            actor_user_id: None,
            actor_principal_id: None,
            action: "share_link.accessed",
            target_id: Some(resolved.share_link_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "phase": "resolve",
                "storage_object_id": resolved.storage_object_id,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(PublicShareResponse {
        share: db::map_public_record_to_view(resolved),
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "Public download URL creation keeps infrastructure dependencies explicit."
)]
pub async fn create_public_download_url(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    storage: &dyn nvbes_storage::ObjectStore,
    token: &str,
    client_key: &str,
    require_clean_scan: bool,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<PublicDownloadUrlResponse, AppError> {
    nvbes_core::limiter::check_rate_limit(
        redis,
        "public_share_download",
        client_key,
        15,
        Duration::from_secs(60),
    )
    .await?;

    let mut tx = db.begin().await?;
    let hash = nvbes_core::auth::token_hash(token.trim());
    set_public_share_rls_context(&mut tx, &hash).await?;
    let resolved = queries::resolve_public_share_for_update_tx(&mut tx, &hash).await?;
    if let Err(error) = logic::enforce_public_share_access(&resolved, require_clean_scan) {
        log_public_share_denied(
            &mut tx,
            &resolved,
            &error.code,
            ip.as_deref(),
            user_agent.as_deref(),
        )
        .await?;
        tx.commit().await?;
        return Err(error);
    }

    if let Err(error) = logic::ensure_public_share_download_allowed(&resolved) {
        log_public_share_denied(
            &mut tx,
            &resolved,
            &error.code,
            ip.as_deref(),
            user_agent.as_deref(),
        )
        .await?;
        tx.commit().await?;
        return Err(error);
    }

    db::increment_download_count_tx(&mut tx, resolved.share_link_id).await?;

    let expires_at = std::cmp::min(
        Utc::now() + ChronoDuration::seconds(60),
        resolved.expires_at,
    );
    let object_key = resolved.object_key.ok_or_else(|| {
        AppError::conflict(
            "missing_object_key",
            "File is missing its storage object key.",
        )
    })?;

    crate::domains::quotas::record_bandwidth_out_tx(
        &mut tx,
        BandwidthOutUsageInput {
            workspace_id: resolved.workspace_id,
            actor_user_id: None,
            actor_principal_id: None,
            storage_object_id: resolved.storage_object_id,
            source: "share_link.download_url_created",
            size_bytes: resolved.size_bytes,
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            idempotency_key: format!(
                "share-download-url:{}:{}",
                resolved.share_link_id,
                Uuid::new_v4()
            ),
        },
    )
    .await?;

    observability::log_share_link_event_tx(
        &mut tx,
        AuditEventInput {
            workspace_id: resolved.workspace_id,
            actor_user_id: None,
            actor_principal_id: None,
            action: "share_link.accessed",
            target_id: Some(resolved.share_link_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "phase": "download_url_created",
                "storage_object_id": resolved.storage_object_id,
                "download_count": resolved.download_count + 1,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    let download_url =
        logic::build_signed_public_download_url(storage, &object_key, expires_at).await?;

    Ok(PublicDownloadUrlResponse {
        share_link_id: resolved.share_link_id,
        download_url,
    })
}

async fn set_public_share_rls_context(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    token_hash: &str,
) -> Result<(), AppError> {
    let workspace_id =
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT resolve_public_share_workspace($1)")
            .bind(token_hash)
            .fetch_one(&mut **tx)
            .await?
            .ok_or_else(|| AppError::not_found("share_link_not_found", "Share link not found."))?;

    nvbes_tenancy::set_transaction_rls_context(
        tx,
        nvbes_tenancy::RlsContext {
            workspace_id: Some(workspace_id),
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

async fn log_public_share_denied(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    resolved: &db::PublicShareRecord,
    reason: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(), AppError> {
    observability::log_share_link_event_tx(
        tx,
        AuditEventInput {
            workspace_id: resolved.workspace_id,
            actor_user_id: None,
            actor_principal_id: None,
            action: "share_link.access_denied",
            target_id: Some(resolved.share_link_id),
            ip,
            user_agent,
            metadata: serde_json::json!({
                "reason": reason,
                "storage_object_id": resolved.storage_object_id,
                "object_status": resolved.object_status.as_str(),
                "scan_status": &resolved.scan_status,
            }),
        },
    )
    .await
}
