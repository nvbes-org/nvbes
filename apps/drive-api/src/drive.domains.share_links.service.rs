use uuid::Uuid;

use crate::{domains::authz::WorkspaceAccess, http::error::AppError};

use super::types::*;
pub use super::{db, logic, observability, queries, types};
use sqlx::PgPool;

#[path = "drive.domains.share_links.manage.rs"]
mod manage;
#[path = "drive.domains.share_links.public.rs"]
mod public;

pub async fn list_share_links(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<ShareLinkListResponse, AppError> {
    manage::list_share_links(db, access).await
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
    manage::create_share_link(
        db,
        access,
        object_id,
        input,
        require_clean_scan,
        ip,
        user_agent,
    )
    .await
}

pub async fn update_share_link(
    db: &PgPool,
    access: &WorkspaceAccess,
    share_link_id: Uuid,
    input: UpdateShareLinkInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ShareLinkResponse, AppError> {
    manage::update_share_link(db, access, share_link_id, input, ip, user_agent).await
}

pub async fn revoke_share_link(
    db: &PgPool,
    access: &WorkspaceAccess,
    share_link_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<ShareLinkResponse, AppError> {
    manage::revoke_share_link(db, access, share_link_id, ip, user_agent).await
}

pub async fn get_public_share(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    token: &str,
    client_key: &str,
    require_clean_scan: bool,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<PublicShareResponse, AppError> {
    public::get_public_share(
        db,
        redis,
        token,
        client_key,
        require_clean_scan,
        ip,
        user_agent,
    )
    .await
}

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
    public::create_public_download_url(
        db,
        redis,
        storage,
        token,
        client_key,
        require_clean_scan,
        ip,
        user_agent,
    )
    .await
}
