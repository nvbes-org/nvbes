use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::auth::types::AuthContext, domains::authz::WorkspaceAccess, http::error::AppError,
};
use nvbes_core::config::AppConfig;

use super::{invites, membership};

pub use super::types::*;

pub async fn list_members(
    db: &PgPool,
    access: &WorkspaceAccess,
) -> Result<MemberListResponse, AppError> {
    membership::list_members(db, access).await
}

pub async fn invite_member(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    access: &WorkspaceAccess,
    input: InviteMemberInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<InviteMemberResponse, AppError> {
    invites::invite_member(db, redis, config, access, input, ip, user_agent).await
}

pub async fn update_member_role(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
    member_id: Uuid,
    input: UpdateMemberInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<UpdateMemberResponse, AppError> {
    membership::update_member_role(db, redis, access, member_id, input, ip, user_agent).await
}

pub async fn remove_member(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
    member_id: Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<RemoveMemberResponse, AppError> {
    membership::remove_member(db, redis, access, member_id, ip, user_agent).await
}

pub async fn accept_invitation(
    db: &PgPool,
    auth: &AuthContext,
    input: AcceptInvitationInput,
    ip: Option<String>,
    user_agent: Option<String>,
) -> Result<AcceptInvitationResponse, AppError> {
    invites::accept_invitation(db, auth, input, ip, user_agent).await
}
