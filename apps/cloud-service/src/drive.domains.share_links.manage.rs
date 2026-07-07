use super::db;
use super::logic;
use super::observability::{self, AuditEventInput};
use super::queries;
use super::types::*;
use crate::{domains::authz::WorkspaceAccess, http::error::AppError};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list_share_links(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<ShareLinkListResponse, AppError> {
    let links = queries::list_share_links(db, access.workspace_id).await?;

    Ok(ShareLinkListResponse {
        share_links: links.into_iter().map(db::map_record_to_view).collect(),
    })
}

pub async fn create_share_link(
    db: &PgPool,
    access: &WorkspaceAccess,
    object_id: Uuid,
    input: CreateShareLinkInput,
    require_clean_scan: bool,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ShareLinkResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let object =
        queries::ensure_shareable_object_tx(&mut tx, access.workspace_id, object_id).await?;
    logic::ensure_shareable_object_for_public_link(&object, require_clean_scan)?;
    let policy = queries::fetch_share_policy_tx(&mut tx, access.workspace_id).await?;

    let expires_at = logic::normalize_expires_at(input.expires_at, &policy)?;
    let max_downloads = logic::normalize_max_downloads(input.max_downloads)?;
    queries::enforce_share_link_capacity_tx(&mut tx, access.workspace_id, policy.max_share_links)
        .await?;

    let token = nvbes_core::auth::generate_token("gxs");
    let token_hash = nvbes_core::auth::token_hash(&token);

    let link = db::insert_share_link_tx(
        &mut tx,
        access.workspace_id,
        object_id,
        &token_hash,
        expires_at,
        max_downloads,
        access.auth.user_id,
        access.auth.principal_id,
    )
    .await?;

    observability::log_share_link_event_tx(
        &mut tx,
        AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "share_link.created",
            target_id: Some(link.id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "storage_object_id": object_id,
                "expires_at": expires_at,
                "max_downloads": max_downloads,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    let share_url = Some(logic::build_public_share_url(link.id, &token));
    let share_link = db::map_record_to_view(link);

    Ok(ShareLinkResponse {
        share_link,
        share_url,
        token: Some(token),
    })
}

pub async fn update_share_link(
    db: &PgPool,
    access: &WorkspaceAccess,
    share_link_id: Uuid,
    input: UpdateShareLinkInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ShareLinkResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let current =
        queries::fetch_share_link_for_update_tx(&mut tx, access.workspace_id, share_link_id)
            .await?;
    logic::ensure_link_not_revoked_or_expired(&current)?;
    let policy = queries::fetch_share_policy_tx(&mut tx, access.workspace_id).await?;

    let expires_at = match input.expires_at {
        Some(value) => logic::normalize_expires_at(Some(value), &policy)?,
        None => current.expires_at,
    };
    let max_downloads = match input.max_downloads {
        Some(value) => logic::normalize_max_downloads(value)?,
        None => current.max_downloads,
    };

    let link = db::update_share_link_tx(
        &mut tx,
        access.workspace_id,
        share_link_id,
        expires_at,
        max_downloads,
    )
    .await?;

    observability::log_share_link_event_tx(
        &mut tx,
        AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "share_link.updated",
            target_id: Some(share_link_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "expires_at": expires_at,
                "max_downloads": max_downloads,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(ShareLinkResponse {
        share_link: db::map_record_to_view(link),
        share_url: None,
        token: None,
    })
}

pub async fn revoke_share_link(
    db: &PgPool,
    access: &WorkspaceAccess,
    share_link_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ShareLinkResponse, AppError> {
    let mut tx = crate::domains::authz::begin_workspace_transaction(db, access).await?;
    let current =
        queries::fetch_share_link_for_update_tx(&mut tx, access.workspace_id, share_link_id)
            .await?;

    if current.revoked_at.is_some() {
        return Err(AppError::conflict(
            "share_link_revoked",
            "Share link is already revoked.",
        ));
    }

    let revoked_at = Utc::now();
    let link =
        db::revoke_share_link_tx(&mut tx, access.workspace_id, share_link_id, revoked_at).await?;

    observability::log_share_link_event_tx(
        &mut tx,
        AuditEventInput {
            workspace_id: access.workspace_id,
            actor_user_id: access.auth.audit_actor_user_id(),
            actor_principal_id: Some(access.auth.principal_id),
            action: "share_link.revoked",
            target_id: Some(share_link_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({
                "revoked_at": revoked_at,
            }),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(ShareLinkResponse {
        share_link: db::map_record_to_view(link),
        share_url: None,
        token: None,
    })
}
